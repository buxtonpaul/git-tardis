use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{
    ActivePanel, AppState, CommitSummary, ModifiedFileEntry, NavigationMode, SidebarView,
};
use git_tardis::git::GitRepo;
use git_tardis::ui::Action;

fn setup_comparison_test_repo() -> (TempDir, PathBuf) {
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
    run(&["config", "user.name", "Test Author"]);
    run(&["config", "user.email", "author@example.com"]);

    // Commit 1: Initial creation of src/main.rs and README.md
    fs::write(
        repo_path.join("README.md"),
        "# Git Tardis\nTime travel code navigator\n",
    )
    .unwrap();
    fs::create_dir_all(repo_path.join("src")).unwrap();
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello v1\");\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Initial commit"]);

    // Commit 2: Update src/main.rs and add src/lib.rs
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello v2\");\n}\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn calculate(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Add lib and update main"]);

    // Commit 3: Expand src/lib.rs
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn calculate(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn multiply(a: i32, b: i32) -> i32 {\n    a * b\n}\n",
    ).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Add multiply function"]);

    // Working directory modifications (uncommitted dirty changes)
    fs::write(
        repo_path.join("README.md"),
        "# Git Tardis\nTime travel code navigator with single GitRepo caching\n",
    )
    .unwrap();
    fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn calculate(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn multiply(a: i32, b: i32) -> i32 {\n    a * b + 1\n}\n",
    ).unwrap();

    (temp_dir, repo_path)
}

fn load_repo_into_app(app: &mut AppState) {
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
            app.commits = commits.into_iter().map(CommitSummary::from).collect();
        }
        app.load_currently_selected_file();
    }
}

#[test]
fn test_single_gitrepo_instance_persistence() {
    let (_dir, repo_path) = setup_comparison_test_repo();
    let mut app = AppState::new(repo_path.clone());

    // Verify AppState created and holds an open GitRepo instance
    assert!(
        app.repo.is_some(),
        "AppState failed to hold single open GitRepo instance"
    );
    assert_eq!(
        app.repo_ref().unwrap().work_dir(),
        GitRepo::open(&repo_path).unwrap().work_dir()
    );

    load_repo_into_app(&mut app);

    // Verify working copy files match baseline GitRepo query
    let baseline_repo = GitRepo::open(&repo_path).unwrap();
    let baseline_files = baseline_repo.list_files().unwrap();
    assert_eq!(
        app.files, baseline_files,
        "File list in AppState differs from baseline GitRepo query"
    );

    // Verify working tree status matches baseline GitRepo status
    let baseline_status = baseline_repo.get_status().unwrap();
    let baseline_dirty: Vec<ModifiedFileEntry> = baseline_status
        .into_iter()
        .map(ModifiedFileEntry::from)
        .collect();
    assert_eq!(
        app.dirty_files, baseline_dirty,
        "Dirty files in AppState differ from baseline GitRepo query"
    );

    // Verify commit history matches baseline GitRepo commit history
    let baseline_commits = baseline_repo.get_commit_history(Some(50)).unwrap();
    let expected_commits: Vec<CommitSummary> = baseline_commits
        .into_iter()
        .map(CommitSummary::from)
        .collect();
    assert_eq!(
        app.commits, expected_commits,
        "Commit timeline in AppState differs from baseline GitRepo query"
    );
}

#[test]
fn test_sidebar_views_and_commit_queries_comparison() {
    let (_dir, repo_path) = setup_comparison_test_repo();
    let baseline_repo = GitRepo::open(&repo_path).unwrap();
    let mut app = AppState::new(repo_path);
    load_repo_into_app(&mut app);

    // 1. File Explorer View Comparison
    app.set_sidebar_view(SidebarView::FileExplorer);
    let visible_items = app.visible_file_items();
    let first_file_idx = visible_items
        .iter()
        .position(|it| !it.is_dir)
        .expect("No file in visible items");
    app.file_selected = first_file_idx;
    app.load_currently_selected_file();
    assert_eq!(
        app.active_file.as_deref(),
        Some(visible_items[first_file_idx].path.as_str()),
        "Active file does not match selected file in explorer"
    );

    // 2. Modified Files View Comparison
    app.set_sidebar_view(SidebarView::ModifiedFiles);
    app.dirty_selected = 0;
    app.load_currently_selected_file();
    let current_dirty = app.dirty_files[0].path.clone();
    assert_eq!(
        app.active_file.as_deref(),
        Some(current_dirty.as_str()),
        "Active file does not match selected dirty file"
    );

    // 3. Commit Timeline Selection Comparison
    app.set_sidebar_view(SidebarView::CommitTimeline);
    app.commit_selected = 1; // "Add lib and update main"
    let target_hash = app.commits[1].hash.clone();

    // Select commit in timeline
    app.dispatch_action(Action::Select);
    assert_eq!(app.selected_commit_hash, Some(target_hash.clone()));

    // Verify modified files at target commit match baseline query
    let baseline_commit_files = baseline_repo.get_commit_files(&target_hash).unwrap();
    let expected_modified: Vec<ModifiedFileEntry> = baseline_commit_files
        .into_iter()
        .map(ModifiedFileEntry::from)
        .collect();
    assert_eq!(
        app.modified_files, expected_modified,
        "Modified files for selected commit differ from baseline query"
    );

    // Verify tree files at target commit match baseline query
    let baseline_tree_files = baseline_repo.list_files_at_commit(&target_hash).unwrap();
    assert_eq!(
        app.files, baseline_tree_files,
        "File explorer list at target commit differs from baseline query"
    );

    // 4. Target Candidates View Comparison
    app.set_sidebar_view(SidebarView::CommitTimeline);
    app.toggle_timeline_filter(); // Switch timeline filter to CANDIDATES
    app.set_navigation_mode(NavigationMode::File);
    let cur_file = app.current_file_path().unwrap();
    let baseline_file_commits = baseline_repo.get_file_commits(&cur_file, None).unwrap();
    let expected_candidates: Vec<CommitSummary> = baseline_file_commits
        .into_iter()
        .map(CommitSummary::from)
        .collect();
    assert_eq!(
        app.candidate_commits, expected_candidates,
        "Candidate commits for file mode differ from baseline query"
    );

    // Reset time travel and compare working state restoration
    app.reset_time_travel();
    assert_eq!(app.selected_commit_hash, None);
    assert_eq!(
        app.files,
        baseline_repo.list_files().unwrap(),
        "Reset time travel did not restore working directory file list"
    );
}

#[test]
fn test_time_travel_jumps_and_blame_state_comparison() {
    let (_dir, repo_path) = setup_comparison_test_repo();
    let baseline_repo = GitRepo::open(&repo_path).unwrap();
    let mut app = AppState::new(repo_path);
    load_repo_into_app(&mut app);

    app.set_sidebar_view(SidebarView::FileExplorer);
    // Find index for src/main.rs in visible file tree items
    let main_idx = app
        .visible_file_items()
        .iter()
        .position(|f| f.path == "src/main.rs")
        .expect("src/main.rs not found");
    app.file_selected = main_idx;
    app.load_currently_selected_file();
    app.active_panel = ActivePanel::CodeViewer;

    // Time travel jump PREVIOUS in FILE mode
    app.set_navigation_mode(NavigationMode::File);
    app.dispatch_action(Action::JumpPrevAuto);

    let jump_hash = app
        .selected_commit_hash
        .clone()
        .expect("Time travel jump failed to set commit hash");
    let baseline_file_content = baseline_repo
        .get_file_at_commit(&jump_hash, "src/main.rs")
        .unwrap();
    let expected_lines: Vec<String> = baseline_file_content
        .lines()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        app.code_lines, expected_lines,
        "Loaded code lines at historical commit differ from baseline query"
    );

    // Verify current line blame matches baseline blame query
    app.cursor_line = 1;
    app.update_current_line_blame();
    let cached_blame = app
        .current_line_blame
        .as_ref()
        .expect("Missing current line blame");
    let baseline_blame_vec = baseline_repo
        .get_blame_at_commit(Some(&jump_hash), "src/main.rs", Some(1), Some(1))
        .unwrap();
    let baseline_blame = baseline_blame_vec
        .first()
        .expect("Empty baseline blame query");

    assert_eq!(
        cached_blame.commit_hash, baseline_blame.commit_hash,
        "Blame commit hash differs from baseline query"
    );
    assert_eq!(
        cached_blame.author, baseline_blame.author,
        "Blame author differs from baseline query"
    );

    // Return from time travel to HEAD/working copy
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(
        app.selected_commit_hash, None,
        "Stepping NEXT past newest commit did not return to working directory"
    );
}
