use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{AppState, FileViewMode, SidebarView};
use git_tardis::ui::{render, Action, KeyDispatcher, KeyStroke, KeymapRegistry};
use ratatui::{backend::TestBackend, Terminal};

fn setup_test_repo() -> (TempDir, PathBuf) {
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
            "git command failed: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    // Commit 1: Initial file
    std::fs::write(
        repo_path.join("file.rs"),
        "fn main() {\n    println!(\"v1\");\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "initial commit"]);

    // Commit 2: Modify file
    std::fs::write(
        repo_path.join("file.rs"),
        "fn main() {\n    println!(\"v2 modified\");\n}\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "modify println"]);

    (temp_dir, repo_path)
}

#[test]
fn test_file_view_mode_toggle_and_diff_display() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path);

    // Initial state: Full view mode
    assert_eq!(app.file_view_mode, FileViewMode::Full);
    if let Some(repo) = app.repo() {
        if let Ok(files) = repo.list_files() {
            app.files = files;
        }
        if let Ok(commits) = repo.get_commit_history(Some(50)) {
            app.commits = commits.into_iter().map(Into::into).collect();
        }
    }

    // Set target commit hash to the second commit
    let target_hash = app.commits[0].hash.clone();
    app.selected_commit_hash = Some(target_hash.clone());
    app.load_currently_selected_file();

    // Verify full file content is loaded
    assert!(app.code_lines.contains(&"fn main() {".to_string()));
    assert!(app
        .code_lines
        .contains(&"    println!(\"v2 modified\");".to_string()));

    // Render UI and verify title has "View: [FULL]"
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg1.contains("View: [FULL]"));

    // Toggle File View Mode (Action::ToggleFileViewMode)
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Diff);
    assert!(
        app.status_message.contains("Loaded working diff")
            || app.status_message.contains("Loaded diff")
    );

    // Verify code lines now show unified git diff
    assert!(app.code_lines.iter().any(|l| l.starts_with("diff --git")));
    assert!(app
        .code_lines
        .iter()
        .any(|l| l.starts_with("-    println!(\"v1\");")
            || l.starts_with("+    println!(\"v2 modified\");")));

    // Render UI and verify title has "View: [DIFF]"
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg2 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg2.contains("View: [DIFF]"));

    // Toggle back to Full view mode
    app.dispatch_action(Action::ToggleFileViewMode);
    assert_eq!(app.file_view_mode, FileViewMode::Full);
    assert!(app.status_message.contains("Loaded file") || app.status_message.contains("Loaded"));
    assert!(app.code_lines.contains(&"fn main() {".to_string()));
}

#[test]
fn test_file_view_mode_keybinding_dispatcher() {
    let registry = KeymapRegistry::new();
    let mut dispatcher = KeyDispatcher::new(registry);

    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path);

    // Pressing 'd' in global scope maps to Action::ToggleFileViewMode
    let action = dispatcher.handle_key(KeyStroke::Char('d'), app.active_scope());
    assert_eq!(action, Some(Action::ToggleFileViewMode));

    if let Some(act) = action {
        app.dispatch_action(act);
    }
    assert_eq!(app.file_view_mode, FileViewMode::Diff);
}

#[test]
fn test_working_directory_diff_and_untracked_files() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path.clone());

    // Create an untracked file in the working directory
    std::fs::write(
        repo_path.join("untracked.rs"),
        "pub fn new_feature() {\n    todo!();\n}\n",
    )
    .unwrap();

    if let Some(repo) = app.repo() {
        if let Ok(files) = repo.list_files() {
            app.files = files;
        }
    }

    // Set sidebar view to ModifiedFiles and set dirty_files to untracked.rs
    app.sidebar_view = SidebarView::ModifiedFiles;
    app.dirty_files = vec![git_tardis::app::ModifiedFileEntry::new(
        "untracked.rs",
        "??",
    )];
    app.dirty_selected = 0;
    app.file_view_mode = FileViewMode::Diff;
    app.load_currently_selected_file();

    // Verify untracked file diff shows additions
    assert!(app.code_lines.iter().any(|l| l.contains("untracked.rs")));
    assert!(app
        .code_lines
        .iter()
        .any(|l| l.starts_with("+pub fn new_feature()")));

    // Set sidebar view to ModifiedFiles with no selected file to test full working directory diff mode
    app.sidebar_view = SidebarView::ModifiedFiles;
    app.active_file = None;
    app.files.clear();
    app.dirty_files.clear();
    app.modified_files.clear();
    app.load_currently_selected_file();

    assert!(app.status_message.contains("working directory diff"));
    assert!(app.code_lines.iter().any(|l| l.contains("untracked.rs")));
}

#[test]
fn test_unmodified_file_with_plus_prefix_gutter() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path);

    app.file_view_mode = FileViewMode::Full;
    app.code_lines = vec![
        "+ line starting with plus".to_string(),
        "regular line".to_string(),
    ];
    app.file_diff_highlights.clear(); // File is unmodified (no diff highlights)

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // Set cursor to line 2 so line 1 renders with regular gutter prefix (not cursor '> ')
    app.cursor_line = 2;
    terminal.draw(|f| render(f, &mut app)).unwrap();

    let buffer = terminal.backend().buffer();
    let dbg = format!("{:?}", buffer);

    // Verify gutter renders "  1   + line starting with plus" and NOT "  1 + + line..."
    assert!(!dbg.contains("1 + + line"));
}

#[test]
fn test_unmodified_tracked_file_has_no_diff_highlights() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path);

    // file.rs is a tracked file in repo_path with no working directory modifications
    app.active_file = Some("file.rs".to_string());
    app.file_view_mode = FileViewMode::Full;
    app.load_currently_selected_file();

    // Verify working diff returned empty and no lines were marked as added
    assert!(app.file_diff_highlights.is_empty());
}

#[test]
fn test_directory_diff_includes_subdirectories() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path.clone());

    // Create a subdirectory structure src/sub/mod.rs
    let sub_dir = repo_path.join("src").join("sub");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(
        sub_dir.join("mod.rs"),
        "pub fn sub_func() {\n    println!(\"sub\");\n}\n",
    )
    .unwrap();

    if let Some(repo) = app.repo() {
        if let Ok(files) = repo.list_files() {
            app.files = files;
        }
    }

    // In Diff mode, select the parent directory "src" in FileExplorer
    app.file_view_mode = FileViewMode::Diff;
    let items = app.visible_file_items();
    if let Some(src_idx) = items.iter().position(|it| it.path == "src") {
        app.file_selected = src_idx;
    } else {
        app.active_file = Some("src".to_string());
    }
    app.load_currently_selected_file();

    // Verify folder diff includes files from nested subdirectories
    assert!(app.code_lines.iter().any(|l| l.contains("src/sub/mod.rs")));
    assert!(app
        .code_lines
        .iter()
        .any(|l| l.starts_with("+pub fn sub_func()")));
}

#[test]
fn test_non_existent_file_status_message_in_diff_view_mode() {
    let (_dir, repo_path) = setup_test_repo();
    let mut app = AppState::new(repo_path);

    if let Some(repo) = app.repo() {
        if let Ok(commits) = repo.get_commit_history(Some(50)) {
            app.commits = commits.into_iter().map(Into::into).collect();
        }
    }

    // setup_test_repo creates commit 1 (initial file.rs) and commit 2 (modifies file.rs).
    // Let's get commit 1's hash (oldest commit).
    let commit1_hash = app.commits.last().unwrap().hash.clone();
    let short_hash = &commit1_hash[..7];

    // Select commit 1 and a file that did not exist at commit 1
    app.selected_commit_hash = Some(commit1_hash.clone());
    app.active_file = Some("non_existent.rs".to_string());

    // Switch to Diff view mode and load file
    app.file_view_mode = FileViewMode::Diff;
    app.load_currently_selected_file();

    let expected_msg = format!("File 'non_existent.rs' did not exist at commit {}", short_hash);

    // Verify status message and code_lines contain non-existent file message in Diff mode
    assert_eq!(app.status_message, expected_msg);
    assert_eq!(app.code_lines, vec![expected_msg.clone()]);

    // Verify terminal rendering displays the message
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer_output = format!("{:?}", terminal.backend().buffer());
    assert!(buffer_output.contains(&expected_msg));
}
