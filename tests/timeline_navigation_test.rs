use git_tardis::app::{ActivePanel, AppState, CommitSummary, NavigationMode, SidebarView};
use git_tardis::git::GitRepo;
use git_tardis::timeline::{JumpDirection, JumpScope, TimelineJumpRequest, TimelineNavigator};
use git_tardis::ui::Action;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn setup_test_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .expect("Failed to execute git command");
        assert!(
            output.status.success(),
            "Git command failed: git {}\nStderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    let repo = GitRepo::open(repo_path).expect("Failed to open initialized git repo");
    (temp_dir, repo)
}

fn commit_file(repo: &GitRepo, filename: &str, content: &str, message: &str) {
    let file_path = repo.work_dir().join(filename);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&file_path, content).expect("Failed to write test file");

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(repo.work_dir())
            .args(args)
            .output()
            .expect("Failed to execute git command");
        assert!(
            output.status.success(),
            "Git command failed: git {}\nStderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["add", filename]);
    run(&["commit", "-m", message]);
}

#[test]
fn test_file_mode_timeline_navigation() {
    let (_dir, repo) = setup_test_repo();

    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v1\"); }\n",
        "C1: v1",
    );
    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v2\"); }\n",
        "C2: v2",
    );
    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v3\"); }\n",
        "C3: v3",
    );

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["main.rs".to_string()];
    app.load_currently_selected_file();
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v3\"); }"]);

    // Jump PREV -> C2 ("v2") on clean working directory
    app.dispatch_action(Action::JumpPrevFile);
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);
    assert!(app.status_message.contains("Jumped [PREVIOUS] (FILE)"));

    // Jump PREV -> C1 ("v1")
    app.dispatch_action(Action::JumpPrevFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v1\"); }"]);

    // Jump PREV again -> bounds error
    app.dispatch_action(Action::JumpPrevFile);
    assert!(app.status_message.contains("Already at oldest commit"));

    // Jump NEXT -> C2 ("v2")
    app.dispatch_action(Action::JumpNextFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);

    // Jump NEXT -> C3 ("v3")
    app.dispatch_action(Action::JumpNextFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v3\"); }"]);

    // Jump NEXT -> working directory
    app.dispatch_action(Action::JumpNextFile);
    assert_eq!(app.selected_commit_hash, None);
    assert!(app.status_message.contains("Returned to working directory"));
}

#[test]
fn test_function_mode_timeline_navigation() {
    let (_dir, repo) = setup_test_repo();

    let code_v1 = r#"fn helper() {
    println!("helper v1");
}

fn target() {
    println!("target v1");
}
"#;

    let code_v2 = r#"fn helper() {
    println!("helper v2");
}

fn target() {
    println!("target v1");
}
"#;

    let code_v3 = r#"fn helper() {
    println!("helper v2");
}

fn target() {
    println!("target v2");
}
"#;

    let code_v4 = r#"fn helper() {
    println!("helper v2");
}

fn target() {
    println!("target v3");
}
"#;

    commit_file(&repo, "lib.rs", code_v1, "C1: Base");
    commit_file(&repo, "lib.rs", code_v2, "C2: Modify helper only");
    commit_file(&repo, "lib.rs", code_v3, "C3: Modify target to v2");
    commit_file(&repo, "lib.rs", code_v4, "C4: Modify target to v3");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["lib.rs".to_string()];
    app.load_currently_selected_file();

    // Set cursor on line 6 (inside `target()`)
    app.cursor_line = 6;

    // Step PREV in Function mode on clean working tree -> jumps from HEAD (C4) directly to C3 (target v2)
    app.dispatch_action(Action::JumpPrevFunction);
    assert!(app.status_message.contains("C3: Modify target to v2"));

    // Step PREV in Function mode -> should skip C2 and hit C1 (Base)
    app.dispatch_action(Action::JumpPrevFunction);
    assert!(app.status_message.contains("C1: Base"));

    // Step PREV again -> bounds error
    app.dispatch_action(Action::JumpPrevFunction);
    assert!(app.status_message.contains("Already at oldest commit"));
}

#[test]
fn test_line_mode_timeline_navigation() {
    let (_dir, repo) = setup_test_repo();

    let text_v1 = "line 1\nline 2\nline 3\n";
    let text_v2 = "line 1 v2\nline 2\nline 3\n";
    let text_v3 = "line 1 v2\nline 2\nline 3 v2\n";

    commit_file(&repo, "data.txt", text_v1, "C1: Initial data");
    commit_file(&repo, "data.txt", text_v2, "C2: Modify line 1");
    commit_file(&repo, "data.txt", text_v3, "C3: Modify line 3");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["data.txt".to_string()];
    app.load_currently_selected_file();

    // Cursor on line 1 -> jumps on clean repo skip C3 (line 3 only) and jump directly to C2 (modify line 1)
    app.cursor_line = 1;

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C2: Modify line 1"));

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C1: Initial data"));

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("Already at oldest commit"));

    // Now test line 3 on clean working copy -> line 3 in HEAD is C3, so JumpPrevLine jumps directly to C1
    app.selected_commit_hash = None;
    app.cursor_line = 3;

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C1: Initial data"));
}

#[test]
fn test_auto_mode_timeline_navigation() {
    let (_dir, repo) = setup_test_repo();

    let code = "fn foo() {\n    println!(\"hello\");\n}\n";
    commit_file(&repo, "test.rs", code, "C1: Add foo");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["test.rs".to_string()];
    app.load_currently_selected_file();

    // Default mode: Commit mode
    assert_eq!(app.nav_mode, NavigationMode::Commit);
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.status_message.contains("COMMIT"));

    // Cycle to File mode
    app.cycle_navigation_mode();
    assert_eq!(app.nav_mode, NavigationMode::File);
    app.selected_commit_hash = None;
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.status_message.contains("FILE"));

    // Cycle to Function mode
    app.cycle_navigation_mode();
    assert_eq!(app.nav_mode, NavigationMode::Function);
    app.selected_commit_hash = None;
    app.cursor_line = 2;
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.status_message.contains("FUNCTION"));

    // Cycle to Line mode
    app.cycle_navigation_mode();
    assert_eq!(app.nav_mode, NavigationMode::Line);
    app.selected_commit_hash = None;
    app.cursor_line = 1;
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.status_message.contains("LINE 1"));
}

#[test]
fn test_timeline_navigator_direct_api() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "foo.txt", "v1\n", "Commit 1");
    commit_file(&repo, "foo.txt", "v2\n", "Commit 2");

    let nav = TimelineNavigator::new();
    let res = nav
        .jump(TimelineJumpRequest {
            repo_path: repo.work_dir(),
            repo: Some(&repo),
            file_path: "foo.txt",
            source_lines: &["v2".to_string()],
            cursor_line: 1,
            current_commit_hash: None,
            head_commit_hash: None,
            scope: JumpScope::File,
            direction: JumpDirection::Previous,
            cached_commits: None,
            is_dirty: false,
        })
        .unwrap()
        .unwrap();

    assert_eq!(res.code_lines, vec!["v1"]);
    assert_eq!(res.commit_summary, "Commit 1");
}

#[test]
fn test_commit_mode_timeline_navigation_and_fallback() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "first.txt", "first file v1\n", "C1: Add first");
    commit_file(&repo, "second.txt", "second file v1\n", "C2: Add second");
    commit_file(&repo, "first.txt", "first file v2\n", "C3: Modify first");

    let history = repo.get_commit_history(None).unwrap();

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["first.txt".to_string(), "second.txt".to_string()];
    app.commits = history.into_iter().map(CommitSummary::from).collect();
    app.load_currently_selected_file();

    // Initially viewing first.txt
    assert_eq!(app.code_lines, vec!["first file v2"]);

    // Jump PREV in Commit mode on clean repo -> jumps directly to C2 ("Add second", first.txt was v1)
    app.dispatch_action(Action::JumpPrevCommit);
    assert_eq!(app.code_lines, vec!["first file v1"]);

    // Jump PREV in Commit mode -> C1 ("Add first")
    app.dispatch_action(Action::JumpPrevCommit);
    assert_eq!(app.code_lines, vec!["first file v1"]);
}

#[test]
fn test_commit_timeline_explorer_navigation_reloads_code() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "app.txt", "app v1\n", "C1: Initial app");
    commit_file(&repo, "app.txt", "app v2\n", "C2: Update app");

    let history = repo.get_commit_history(None).unwrap();

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["app.txt".to_string()];
    app.commits = history.into_iter().map(CommitSummary::from).collect();
    app.sidebar_view = git_tardis::app::SidebarView::CommitTimeline;
    app.file_selected = 0;
    app.load_currently_selected_file();

    // Initially selected commit 0 (C2)
    app.update_modified_files_for_selected_commit();
    app.load_currently_selected_file();
    assert_eq!(app.code_lines, vec!["app v2"]);

    // Move selection down in CommitTimeline -> commit 1 (C1)
    app.move_selection_down();
    assert_eq!(app.commit_selected, 1);
    assert_eq!(app.code_lines, vec!["app v1"]);
}

#[test]
fn test_candidate_commits_sidebar_navigation() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "file_a.txt", "v1\n", "C1: Add file_a");
    commit_file(&repo, "file_b.txt", "v1\n", "C2: Add file_b");
    commit_file(&repo, "file_a.txt", "v2\n", "C3: Update file_a");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["file_a.txt".to_string(), "file_b.txt".to_string()];
    app.load_currently_selected_file();

    // Switch to File mode
    app.set_navigation_mode(NavigationMode::File);
    // Switch to Timeline view with Candidates filter
    app.set_sidebar_view(git_tardis::app::SidebarView::CommitTimeline);
    app.timeline_filter = git_tardis::app::TimelineFilter::Candidates;
    app.update_candidate_commits();

    // Candidates in File mode for file_a.txt should only include C3 and C1 (2 commits)
    assert_eq!(app.candidate_commits.len(), 2);
    assert!(app.candidate_commits[0].message.contains("C3"));
    assert!(app.candidate_commits[1].message.contains("C1"));

    // Navigating down in TargetCandidates switches to candidate 1 (C1)
    app.move_selection_down();
    assert_eq!(app.candidate_selected, 1);
    assert_eq!(app.code_lines, vec!["v1"]);

    // Switching to Commit mode updates candidates to all 3 commits
    app.set_navigation_mode(NavigationMode::Commit);
    assert_eq!(app.candidate_commits.len(), 3);
}

#[test]
fn test_changing_selected_file_updates_candidate_commits() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "file_a.txt", "v1\n", "C1: Add file_a");
    commit_file(&repo, "file_b.txt", "v1\n", "C2: Add file_b");
    commit_file(&repo, "file_a.txt", "v2\n", "C3: Update file_a");
    commit_file(&repo, "file_b.txt", "v2\n", "C4: Update file_b");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["file_a.txt".to_string(), "file_b.txt".to_string()];
    app.set_navigation_mode(NavigationMode::File);
    app.file_selected = 0;
    app.load_currently_selected_file();

    // Candidates for file_a.txt: C3, C1
    assert_eq!(app.candidate_commits.len(), 2);
    assert!(app
        .candidate_commits
        .iter()
        .any(|c| c.message.contains("C3")));
    assert!(app
        .candidate_commits
        .iter()
        .any(|c| c.message.contains("C1")));

    // Move selection down in FileExplorer to file_b.txt
    app.move_selection_down();
    assert_eq!(app.file_selected, 1);

    // Candidate commits list should automatically update to file_b.txt candidates (C4, C2)
    assert_eq!(app.candidate_commits.len(), 2);
    assert!(app
        .candidate_commits
        .iter()
        .any(|c| c.message.contains("C4")));
    assert!(app
        .candidate_commits
        .iter()
        .any(|c| c.message.contains("C2")));
}

#[test]
fn test_reproduce_user_timeline_jump_selection_sync() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "file_a.txt", "v1\n", "C1: file_a v1");
    commit_file(&repo, "file_b.txt", "v1\n", "C2: file_b v1");
    commit_file(&repo, "file_a.txt", "v2\n", "C3: file_a v2");
    commit_file(&repo, "file_a.txt", "v3\n", "C4: file_a v3");

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.reload_repo_data();
    // 1. Navigate to file_a.txt in FileExplorer
    app.sidebar_view = SidebarView::FileExplorer;
    app.file_selected = 0;
    app.load_currently_selected_file();
    assert_eq!(app.current_file_path().as_deref(), Some("file_a.txt"));

    // 2. Switch to CodeViewer / File view and toggle commit timeline
    app.active_panel = ActivePanel::CodeViewer;
    app.set_sidebar_view(SidebarView::CommitTimeline);

    // 3. Switch timeline navigation mode to File
    app.set_navigation_mode(NavigationMode::File);

    let initial_candidates = app.display_candidate_commits();
    println!("Initial candidates (len={}):", initial_candidates.len());
    for (i, c) in initial_candidates.iter().enumerate() {
        println!("  [{}] {} {}", i, c.short_hash, c.message);
    }
    println!("Initial candidate_selected: {}", app.candidate_selected);

    // 4. Press '[' (JumpPrevAuto or JumpPrevFile)
    app.dispatch_action(Action::JumpPrevAuto);

    println!("\nAfter JumpPrevAuto:");
    println!("selected_commit_hash: {:?}", app.selected_commit_hash);
    println!("commit_selected: {}", app.commit_selected);
    println!("candidate_selected: {}", app.candidate_selected);

    let updated_candidates = app.display_candidate_commits();
    for (i, c) in updated_candidates.iter().enumerate() {
        let is_sel = i == app.candidate_selected;
        println!(
            "  [{}] {} {} {}",
            i,
            if is_sel { "=>" } else { "  " },
            c.short_hash,
            c.message
        );
    }

    let target_hash = app.selected_commit_hash.clone().unwrap();
    let sel_commit = &updated_candidates[app.candidate_selected];
    assert!(
        sel_commit.matches_hash(&target_hash),
        "Expected candidate_selected ({}) to match target_hash {}, got {}",
        app.candidate_selected,
        target_hash,
        sel_commit.hash
    );
}

#[test]
fn test_code_viewer_focused_navigation_updates_commit_and_code() {
    let (_dir, repo) = setup_test_repo();

    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v1\"); }\n",
        "Commit 1",
    );
    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v2\"); }\n",
        "Commit 2",
    );
    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v3\"); }\n",
        "Commit 3",
    );

    let history = repo.get_commit_history(None).unwrap();

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["main.rs".to_string()];
    app.commits = history.into_iter().map(CommitSummary::from).collect();
    app.load_currently_selected_file();

    // Toggle focus to CodeViewer
    app.toggle_panel_focus();
    assert_eq!(app.active_panel, git_tardis::app::ActivePanel::CodeViewer);

    // Jump PREV from CodeViewer scope on clean repo -> jumps directly to Commit 2 ("v2")
    app.dispatch_action(Action::JumpPrevFile);

    // Check that CodeViewer content, selected_commit_hash, AND Tab 3 commit_selected are updated!
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);
    assert_eq!(app.commit_selected, 1); // Index 1 in commits list is Commit 2
    assert_eq!(app.modified_files.len(), 1);
    assert!(app.modified_files[0].path.contains("main.rs"));

    // Jump PREV again from CodeViewer scope -> Commit 1 ("v1")
    app.dispatch_action(Action::JumpPrevFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v1\"); }"]);
    assert_eq!(app.commit_selected, 2); // Index 2 in commits list is Commit 1

    // Jump NEXT back towards HEAD -> Commit 2 ("v2")
    app.dispatch_action(Action::JumpNextFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);
    assert_eq!(app.commit_selected, 1);
    assert_eq!(app.commit_selected, 1);
}

#[test]
fn test_sidebar_commit_list_views_bracket_navigation() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "file_a.txt", "v1\n", "C1: Add file_a");
    commit_file(&repo, "file_b.txt", "v1\n", "C2: Add file_b");
    commit_file(&repo, "file_a.txt", "v2\n", "C3: Update file_a");

    let history = repo.get_commit_history(None).unwrap();

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.files = vec!["file_a.txt".to_string(), "file_b.txt".to_string()];
    app.commits = history.into_iter().map(CommitSummary::from).collect();
    app.file_selected = 0;
    app.load_currently_selected_file();

    // Verify keymap dispatcher resolves '[' and ']' in Scope::Sidebar
    let registry = git_tardis::ui::KeymapRegistry::new();
    let mut dispatcher = git_tardis::ui::KeyDispatcher::new(registry);
    assert_eq!(
        dispatcher.handle_key(
            git_tardis::ui::KeyStroke::Char('['),
            git_tardis::ui::Scope::Sidebar
        ),
        Some(Action::JumpPrevAuto)
    );
    assert_eq!(
        dispatcher.handle_key(
            git_tardis::ui::KeyStroke::Char(']'),
            git_tardis::ui::Scope::Sidebar
        ),
        Some(Action::JumpNextAuto)
    );

    // Set Navigation Mode to FILE
    app.set_navigation_mode(NavigationMode::File);
    assert_eq!(app.active_panel, git_tardis::app::ActivePanel::Sidebar);

    // 1. In CommitTimeline view with Candidates filter (File mode candidates for file_a.txt: C3 and C1)
    app.set_sidebar_view(git_tardis::app::SidebarView::CommitTimeline);
    app.timeline_filter = git_tardis::app::TimelineFilter::Candidates;
    app.update_candidate_commits();
    assert_eq!(app.candidate_commits.len(), 2);
    assert!(app.candidate_commits[0].message.contains("C3"));
    assert!(app.candidate_commits[1].message.contains("C1"));

    // Press '[' in Sidebar on clean repo -> JumpPrevAuto in File mode -> jumps directly to C1 (skipping C2 because C2 didn't touch file_a.txt)
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(app.candidate_selected, 1); // Pointing to C1
    assert_eq!(app.code_lines, vec!["v1"]);

    // Press ']' -> jumps back to C3
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.candidate_selected, 0); // Pointing to C3
    assert_eq!(
        app.selected_commit_hash,
        Some(app.candidate_commits[0].hash.clone())
    );
    assert_eq!(app.code_lines, vec!["v2"]);

    // Press ']' again -> reset time travel (return to working copy)
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash, None);

    // 2. In CommitTimeline view in Sidebar with File mode still active on clean repo
    app.set_sidebar_view(git_tardis::app::SidebarView::CommitTimeline);
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.code_lines, vec!["v1"]);

    // Reset back
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(
        app.selected_commit_hash,
        Some(app.candidate_commits[0].hash.clone())
    );
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash, None);
}

#[test]
fn test_sidebar_commit_timeline_autoscroll_tracks_target_candidate() {
    let (_dir, repo) = setup_test_repo();

    for i in 1..=20 {
        commit_file(
            &repo,
            "main.rs",
            &format!("fn main() {{ println!(\"v{}\"); }}\n", i),
            &format!("Commit {}", i),
        );
    }

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.active_file = Some("main.rs".to_string());
    if let Some(r) = app.repo() {
        if let Ok(commits) = r.get_commit_history(Some(50)) {
            app.commits = commits.into_iter().map(Into::into).collect();
        }
    }
    app.set_sidebar_view(git_tardis::app::SidebarView::CommitTimeline);
    app.timeline_filter = git_tardis::app::TimelineFilter::All;

    // Initially at HEAD (Commit 20), commit_selected = 0
    assert_eq!(app.commit_selected, 0);

    // Jump to Commit 5 (which is index 15 in reverse-chronological order)
    let c5_hash = app
        .commits
        .iter()
        .find(|c| c.message == "Commit 5")
        .unwrap()
        .hash
        .clone();
    app.update_state_for_commit_hash(c5_hash);

    assert_eq!(app.commit_selected, 15);

    // Draw UI with small backend height (30x12)
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| git_tardis::ui::render(f, &mut app))
        .unwrap();
    let buffer_str = format!("{:?}", terminal.backend().buffer());

    // Verify Commit 5 is automatically scrolled into view in the rendered sidebar
    assert!(
        buffer_str.contains("Commit 5"),
        "Commit 5 should be automatically scrolled into view in the sidebar"
    );
}

#[test]
fn test_unified_dirty_and_head_commit_timeline_navigation() {
    let (_dir, repo) = setup_test_repo();

    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v1\"); }\n",
        "Commit 1",
    );
    commit_file(
        &repo,
        "main.rs",
        "fn main() { println!(\"v2\"); }\n",
        "Commit 2",
    );

    let mut app = AppState::new(repo.work_dir().to_path_buf());
    app.reload_repo_data();

    // 1. Clean working directory state (no dirty files):
    // Pressing '[' ONCE jumps directly from HEAD (Commit 2) to HEAD~1 (Commit 1)
    app.set_navigation_mode(git_tardis::app::NavigationMode::File);
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash, Some(app.commits[1].hash.clone()));
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v1\"); }"]);

    // Reset back to HEAD (JumpNextAuto to Commit 2, then JumpNextAuto to HEAD)
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash, Some(app.commits[0].hash.clone()));
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash, None);

    // 2. Create an uncommitted change in working directory
    fs::write(
        repo.work_dir().join("main.rs"),
        "fn main() { println!(\"dirty\"); }\n",
    )
    .unwrap();
    app.reload_repo_data();

    // Dirty working directory state (uncommitted changes exist):
    // display_commits() includes virtual DIRTY entry at index 0, followed by HEAD (Commit 2)
    let dirty_commits = app.display_commits();
    assert_eq!(dirty_commits.len(), 3);
    assert!(dirty_commits[0].is_dirty());
    assert_eq!(dirty_commits[1].message, "Commit 2");
    assert_eq!(dirty_commits[2].message, "Commit 1");

    // Press '[' first time -> transitions from DIRTY (working copy) to HEAD Commit 2
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash, Some(app.commits[0].hash.clone()));
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);

    // Press '[' second time -> transitions from HEAD Commit 2 to Commit 1
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash, Some(app.commits[1].hash.clone()));
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v1\"); }"]);

    // Reset back
    app.reset_time_travel();

    // 3. Focus Sidebar CommitTimeline navigation
    app.active_panel = git_tardis::app::ActivePanel::Sidebar;
    app.set_sidebar_view(git_tardis::app::SidebarView::CommitTimeline);
    app.timeline_filter = git_tardis::app::TimelineFilter::All;

    // Initially at DIRTY entry (index 0)
    assert_eq!(app.commit_selected, 0);
    assert_eq!(app.selected_commit_hash, None);

    // Move selection down -> transitions from DIRTY (index 0) to HEAD Commit 2 (index 1)
    app.move_selection_down();
    assert_eq!(app.commit_selected, 1);
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(app.selected_commit_hash, Some(app.commits[0].hash.clone()));

    // Move selection down -> transitions from HEAD (index 1) to Commit 1 (index 2)
    app.move_selection_down();
    assert_eq!(app.commit_selected, 2);
    assert_eq!(app.selected_commit_hash, Some(app.commits[1].hash.clone()));

    // Move selection up -> transitions back to HEAD (index 1)
    app.move_selection_up();
    assert_eq!(app.commit_selected, 1);

    // Move selection up -> transitions back to DIRTY (index 0)
    app.move_selection_up();
    assert_eq!(app.commit_selected, 0);
    assert_eq!(app.selected_commit_hash, None);
}
