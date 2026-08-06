use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{
    ActivePanel, AppState, FileViewMode, ModifiedFileEntry, NavigationMode, SidebarView,
    TimelineFilter,
};
use git_tardis::ui::Action;

/// Creates a rich 8-commit git repository with multiple files, functions, file deletions,
/// and uncommitted dirty working copy changes.
fn setup_matrix_test_repo() -> (TempDir, PathBuf, Vec<String>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_path_buf();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&repo_path)
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
    run(&["config", "user.name", "Matrix Tester"]);
    run(&["config", "user.email", "matrix@example.com"]);

    let mut commit_hashes = Vec::new();

    let get_head_hash = || {
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    // Commit 1: Initial commit with README.md, src/main.rs, src/lib.rs
    fs::write(
        repo_path.join("README.md"),
        "# Matrix Test Project\nVersion 1.0\n",
    )
    .unwrap();
    fs::create_dir_all(repo_path.join("src")).unwrap();
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello World v1\");\n}\n\nfn helper_one() {\n    let x = 10;\n    println!(\"Helper 1: {}\", x);\n}\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C1: Initial commit"]);
    commit_hashes.push(get_head_hash());

    // Commit 2: Update src/lib.rs and add docs/architecture.md
    fs::create_dir_all(repo_path.join("docs")).unwrap();
    fs::write(
        repo_path.join("docs/architecture.md"),
        "# Architecture\nDescribes system architecture.\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn subtract(a: i32, b: i32) -> i32 {\n    a - b\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C2: Add docs and subtract function"]);
    commit_hashes.push(get_head_hash());

    // Commit 3: Modify src/main.rs helper_one and add helper_two
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello World v2\");\n}\n\nfn helper_one() {\n    let x = 20;\n    println!(\"Helper 1 updated: {}\", x);\n}\n\nfn helper_two() {\n    println!(\"Helper 2 added\");\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&[
        "commit",
        "-m",
        "C3: Update main.rs helper_one and add helper_two",
    ]);
    commit_hashes.push(get_head_hash());

    // Commit 4: Add src/utils.rs
    fs::write(
        repo_path.join("src/utils.rs"),
        "pub fn format_greeting(name: &str) -> String {\n    format!(\"Hello, {}!\", name)\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C4: Add utils.rs"]);
    commit_hashes.push(get_head_hash());

    // Commit 5: Update README.md and src/lib.rs
    fs::write(
        repo_path.join("README.md"),
        "# Matrix Test Project\nVersion 2.0 with utilities\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b + 0\n}\n\npub fn subtract(a: i32, b: i32) -> i32 {\n    a - b\n}\n\npub fn multiply(a: i32, b: i32) -> i32 {\n    a * b\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C5: Update README and lib multiply"]);
    commit_hashes.push(get_head_hash());

    // Commit 6: Remove src/utils.rs and update src/main.rs
    run(&["rm", "src/utils.rs"]);
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello World v3\");\n}\n\nfn helper_one() {\n    let x = 30;\n    println!(\"Helper 1 updated v3: {}\", x);\n}\n\nfn helper_two() {\n    println!(\"Helper 2 updated v3\");\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C6: Remove utils.rs and update main.rs"]);
    commit_hashes.push(get_head_hash());

    // Commit 7: Update docs/architecture.md
    fs::write(
        repo_path.join("docs/architecture.md"),
        "# Architecture\nDescribes system architecture v2.\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C7: Update architecture.md"]);
    commit_hashes.push(get_head_hash());

    // Commit 8: Add config.json
    fs::write(repo_path.join("config.json"), "{\"version\": 1}\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C8: Add config.json"]);
    commit_hashes.push(get_head_hash());

    // Uncommitted dirty working tree changes
    fs::write(
        repo_path.join("src/main.rs"),
        "// DIRTY MODIFICATION\nfn main() {\n    println!(\"Hello World v3\");\n}\n\nfn helper_one() {\n    let x = 30;\n    println!(\"Helper 1 updated v3: {}\", x);\n}\n\nfn helper_two() {\n    println!(\"Helper 2 updated v3\");\n}\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("README.md"),
        "# Matrix Test Project DIRTY\nVersion 2.0 with utilities\n",
    )
    .unwrap();
    fs::write(repo_path.join("untracked.txt"), "untracked file content\n").unwrap();

    (temp_dir, repo_path, commit_hashes)
}

fn create_app_state_for_repo(repo_path: PathBuf) -> AppState {
    let mut app = AppState::new(repo_path);
    if let Some(repo) = app.repo() {
        if let Ok(files) = repo.list_files() {
            app.files = files;
            app.expand_all_folders();
        }
        if let Ok(statuses) = repo.get_status() {
            let items: Vec<ModifiedFileEntry> =
                statuses.into_iter().map(ModifiedFileEntry::from).collect();
            app.dirty_files = items;
        }
        if let Ok(commits) = repo.get_commit_history(Some(50)) {
            app.commits = commits
                .into_iter()
                .map(git_tardis::app::CommitSummary::from)
                .collect();
        }
        app.load_currently_selected_file();
    }
    app
}

/// Helper to reliably select a file in FileExplorer view or set active_file
fn select_file(app: &mut AppState, path: &str) {
    app.sidebar_view = SidebarView::FileExplorer;
    app.expand_all_folders();
    let items = app.visible_file_items();
    if let Some(idx) = items.iter().position(|item| item.path == path) {
        app.file_selected = idx;
        app.load_currently_selected_file();
    } else {
        app.active_file = Some(path.to_string());
        app.load_currently_selected_file();
    }
}

#[test]
fn test_sidebar_view_transitions_and_commit_list_filtering() {
    let (_dir, repo_path, commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    // Initial state check
    assert_eq!(app.sidebar_view, SidebarView::FileExplorer);
    assert_eq!(app.timeline_filter, TimelineFilter::All);
    assert_eq!(app.nav_mode, NavigationMode::Commit);

    // Verify commit list count: 8 git commits + 1 dirty working directory entry = 9 entries
    let commits_all = app.display_commits();
    assert_eq!(commits_all.len(), 9);
    assert!(commits_all[0].is_dirty());

    // Switch to SidebarView::ModifiedFiles
    app.dispatch_action(Action::SetSidebarView(2));
    assert_eq!(app.sidebar_view, SidebarView::ModifiedFiles);
    assert_eq!(app.dirty_files.len(), 3); // main.rs, README.md, untracked.txt

    // Switch to SidebarView::CommitTimeline
    app.dispatch_action(Action::SetSidebarView(3));
    assert_eq!(app.sidebar_view, SidebarView::CommitTimeline);

    // By default in TimelineFilter::All, commit list returns all 9 entries
    assert_eq!(app.display_commits().len(), 9);

    // Select src/main.rs explicitly
    select_file(&mut app, "src/main.rs");
    app.dispatch_action(Action::SetSidebarView(3)); // switch back to CommitTimeline
    app.set_navigation_mode(NavigationMode::File);

    // Check candidate commits for src/main.rs (dirty entry + 3 git commits = 4 entries)
    let candidate_commits = app.display_candidate_commits();
    assert_eq!(candidate_commits.len(), 4);

    // Ensure display_commits() (TimelineFilter::All) STILL displays all 9 commits regardless of nav_mode
    assert_eq!(app.display_commits().len(), 9);

    // Toggle filter to TimelineFilter::Candidates
    app.dispatch_action(Action::ToggleTimelineFilter);
    assert_eq!(app.timeline_filter, TimelineFilter::Candidates);

    // Now moving selection down should cycle through candidates
    app.active_panel = ActivePanel::Sidebar;
    app.dispatch_action(Action::MoveDown);
    // Move from dirty to C6 (first historical commit touching main.rs)
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(
        app.selected_commit_hash.as_ref().unwrap(),
        &commit_hashes[5]
    ); // C6 is index 5

    // Toggle back to TimelineFilter::All
    app.dispatch_action(Action::ToggleTimelineFilter);
    assert_eq!(app.timeline_filter, TimelineFilter::All);
    assert_eq!(app.display_commits().len(), 9);
}

#[test]
fn test_navigation_mode_candidate_queries_across_views() {
    let (_dir, repo_path, commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    // Select src/main.rs
    select_file(&mut app, "src/main.rs");

    // 1. NavigationMode::Commit
    app.set_navigation_mode(NavigationMode::Commit);
    app.update_candidate_commits();
    // 8 git commits in candidate_commits
    assert_eq!(app.candidate_commits.len(), 8);
    // 9 entries (including dirty) in display_candidate_commits()
    assert_eq!(app.display_candidate_commits().len(), 9);

    // 2. NavigationMode::File
    app.set_navigation_mode(NavigationMode::File);
    app.update_candidate_commits();
    // main.rs changed in C6, C3, C1 (3 git commits)
    assert_eq!(app.candidate_commits.len(), 3);
    // display_candidate_commits() has 4 (dirty + 3 git commits)
    assert_eq!(app.display_candidate_commits().len(), 4);

    // Switch selected file to src/lib.rs
    select_file(&mut app, "src/lib.rs");
    app.set_navigation_mode(NavigationMode::File);
    app.update_candidate_commits();
    // lib.rs changed in C5, C2, C1 (3 git commits)
    assert_eq!(app.candidate_commits.len(), 3);

    // 3. NavigationMode::Line
    select_file(&mut app, "src/main.rs");
    app.cursor_line = 6; // helper_one line in main.rs
    app.set_navigation_mode(NavigationMode::Line);
    app.update_candidate_commits();
    assert!(!app.candidate_commits.is_empty());

    // 4. NavigationMode::Function
    app.set_navigation_mode(NavigationMode::Function);
    app.update_candidate_commits();
    assert!(!app.candidate_commits.is_empty());

    // Jump prev candidate in Function mode
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.selected_commit_hash.is_some());
    let current_hash = app.selected_commit_hash.clone().unwrap();
    assert!(
        current_hash == commit_hashes[5]
            || current_hash == commit_hashes[2]
            || current_hash == commit_hashes[0]
    );
}

#[test]
fn test_file_view_mode_diff_vs_full_with_target_commit_and_file_switches() {
    let (_dir, repo_path, commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    select_file(&mut app, "src/main.rs");

    // Default FileViewMode is Full
    assert_eq!(app.file_view_mode, FileViewMode::Full);
    assert_eq!(app.code_lines[0], "// DIRTY MODIFICATION");

    // Toggle to Diff mode
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Diff);

    // Code lines in diff mode should contain diff headers/hunks
    assert!(!app.code_lines.is_empty());
    assert!(app
        .code_lines
        .iter()
        .any(|line| line.contains("diff --git") || line.contains("@@")));

    // Move cursor down in diff view
    app.active_panel = ActivePanel::CodeViewer;
    app.dispatch_action(Action::MoveDown);
    app.dispatch_action(Action::MoveDown);

    // Switch active file to README.md while in Diff mode
    select_file(&mut app, "README.md");

    assert_eq!(app.file_view_mode, FileViewMode::Diff);
    assert!(!app.code_lines.is_empty());

    // Switch target commit to C8
    app.update_state_for_commit_hash(commit_hashes[7].clone());
    app.load_currently_selected_file();

    // Reset to HEAD
    app.reset_time_travel();
    assert!(app.selected_commit_hash.is_none());

    // Toggle back to Full mode
    select_file(&mut app, "src/main.rs");
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Full);
}

#[test]
fn test_target_commit_changes_synchronize_modified_files_and_file_tree() {
    let (_dir, repo_path, commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    // At HEAD/DIRTY state
    assert!(app.selected_commit_hash.is_none());
    assert_eq!(app.dirty_files.len(), 3);

    // Travel to C3 (Update main.rs helper_one and add helper_two)
    app.update_state_for_commit_hash(commit_hashes[2].clone());
    assert_eq!(
        app.selected_commit_hash.as_ref().unwrap(),
        &commit_hashes[2]
    );

    // Modified files at C3 should list main.rs
    assert_eq!(app.modified_files.len(), 1);
    assert_eq!(app.modified_files[0].path, "src/main.rs");

    // Travel to C5 (Update README and lib multiply)
    app.update_state_for_commit_hash(commit_hashes[4].clone());
    assert_eq!(
        app.selected_commit_hash.as_ref().unwrap(),
        &commit_hashes[4]
    );

    // Modified files at C5 should list README.md and src/lib.rs
    assert_eq!(app.modified_files.len(), 2);
    let paths: Vec<String> = app.modified_files.iter().map(|f| f.path.clone()).collect();
    assert!(paths.contains(&"README.md".to_string()));
    assert!(paths.contains(&"src/lib.rs".to_string()));

    // Reset back to HEAD / Dirty state
    app.reset_time_travel();
    assert!(app.selected_commit_hash.is_none());
    assert_eq!(app.dirty_files.len(), 3);
}

#[test]
fn test_current_file_and_target_commit_interaction_matrix() {
    let (_dir, repo_path, commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    // Travel to C4 (where src/utils.rs was added)
    app.update_state_for_commit_hash(commit_hashes[3].clone());
    app.sidebar_view = SidebarView::ModifiedFiles;
    if let Some(idx) = app
        .modified_files
        .iter()
        .position(|f| f.path == "src/utils.rs")
    {
        app.modified_selected = idx;
    }
    app.load_currently_selected_file();

    // Now src/utils.rs should be found and loaded
    assert!(!app.code_lines.is_empty());
    assert!(app.code_lines[0].contains("pub fn format_greeting"));

    // Travel to HEAD (where src/utils.rs was deleted in C6)
    app.reset_time_travel();
    app.active_file = Some("src/utils.rs".to_string());
    app.sidebar_view = SidebarView::CommitTimeline;
    app.load_currently_selected_file();

    // At HEAD, src/utils.rs is deleted, so status message indicates non-existent or empty
    assert!(
        app.code_lines.is_empty()
            || app.status_message.contains("not found")
            || app.status_message.contains("Non-existent")
    );
}

#[test]
fn test_complex_end_to_end_user_journey_matrix() {
    let (_dir, repo_path, _commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    // Step 1: Start at DIRTY state in FileExplorer with src/main.rs selected
    select_file(&mut app, "src/main.rs");
    assert_eq!(app.sidebar_view, SidebarView::FileExplorer);

    // Step 2: Set cursor to line 6 (inside helper_one)
    app.cursor_line = 6;
    app.active_panel = ActivePanel::CodeViewer;

    // Step 3: Switch navigation mode to Function
    app.set_navigation_mode(NavigationMode::Function);
    assert_eq!(app.nav_mode, NavigationMode::Function);

    // Step 4: Jump backward through candidate commits touching helper_one
    app.dispatch_action(Action::JumpPrevAuto);
    assert!(app.selected_commit_hash.is_some());
    let _first_jump_hash = app.selected_commit_hash.clone().unwrap();

    // Step 5: Toggle to Diff view mode
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Diff);

    // Step 6: Switch sidebar view to CommitTimeline
    app.dispatch_action(Action::SetSidebarView(3));
    assert_eq!(app.sidebar_view, SidebarView::CommitTimeline);

    // Step 7: Verify display_commits() retains all 9 commits regardless of nav mode
    let commits_all = app.display_commits();
    assert_eq!(commits_all.len(), 9);

    // Step 8: Toggle timeline filter to Candidates
    app.dispatch_action(Action::ToggleTimelineFilter);
    assert_eq!(app.timeline_filter, TimelineFilter::Candidates);

    // Candidate commits list should only contain commits modifying main.rs/helper_one
    let candidates = app.display_candidate_commits();
    assert!(!candidates.is_empty());
    assert!(candidates.len() <= commits_all.len());

    // Step 9: Reset to HEAD
    app.reset_time_travel();
    assert!(app.selected_commit_hash.is_none());

    // Toggle back to Full view
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Full);
}

#[test]
fn test_ui_rendering_for_timeline_and_diff_modes_matrix() {
    use git_tardis::ui::render;
    use ratatui::{backend::TestBackend, Terminal};

    let (_dir, repo_path, _commit_hashes) = setup_matrix_test_repo();
    let mut app = create_app_state_for_repo(repo_path);

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Render in FileExplorer
    select_file(&mut app, "src/main.rs");
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg1.contains("1: Explorer"));
    assert!(dbg1.contains("main.rs"));

    // 2. Render in CommitTimeline [ALL]
    app.dispatch_action(Action::SetSidebarView(3));
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg2 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg2.contains("3: Commit Timeline [ALL]"));
    assert!(dbg2.contains("*DIRTY*") || dbg2.contains("Working Directory"));

    // 3. Render in CommitTimeline [CANDIDATES] in NavigationMode::File
    app.set_navigation_mode(NavigationMode::File);
    app.dispatch_action(Action::ToggleTimelineFilter);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg3 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg3.contains("3: Commit Timeline [CANDIDATES]"));

    // 4. Render in Diff mode
    app.dispatch_action(Action::ToggleFileViewMode);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg4 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg4.contains("DIFF"));
}
