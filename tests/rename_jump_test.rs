use git_tardis::app::{ActivePanel, AppState, NavigationMode, SidebarView};
use git_tardis::git::GitRepo;
use git_tardis::ui::Action;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

const V1: &str = "fn alpha() {\n    let a = 1;\n}\n\nfn beta() {\n    let b = 1;\n}\n";
const V2: &str = "fn alpha() {\n    let a = 2;\n}\n\nfn beta() {\n    let b = 1;\n}\n";
const V4: &str = "// moved\nfn alpha() {\n    let a = 2;\n}\n\nfn beta() {\n    let b = 4;\n}\n";

/// History of one file that is renamed part-way through:
///   Commit 1  creates old.rs (and a.txt)
///   Commit 2  changes `alpha` in old.rs (and a.txt)
///   Commit 3  moves old.rs to src/new.rs, unchanged (and adds aaa/extra.txt)
///   Commit 4  adds a line at the top and changes `beta` in src/new.rs
fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q"]);
    git(p, &["config", "user.name", "Test User"]);
    git(p, &["config", "user.email", "test@example.com"]);

    std::fs::write(p.join("old.rs"), V1).unwrap();
    std::fs::write(p.join("a.txt"), "first\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "Commit 1"]);

    std::fs::write(p.join("old.rs"), V2).unwrap();
    std::fs::write(p.join("a.txt"), "second\n").unwrap();
    git(p, &["commit", "-q", "-am", "Commit 2"]);

    // A new directory sorting ahead of src/ shifts explorer rows between the two trees.
    std::fs::create_dir(p.join("aaa")).unwrap();
    std::fs::write(p.join("aaa/extra.txt"), "extra\n").unwrap();
    git(p, &["add", "."]);
    std::fs::create_dir(p.join("src")).unwrap();
    git(p, &["mv", "old.rs", "src/new.rs"]);
    git(p, &["commit", "-q", "-m", "Commit 3"]);

    std::fs::write(p.join("src/new.rs"), V4).unwrap();
    git(p, &["commit", "-q", "-am", "Commit 4"]);
    dir
}

fn lines(text: &str) -> Vec<String> {
    text.lines().map(|l| l.to_string()).collect()
}

/// Open src/new.rs with the cursor on `let a = 2;`, whose history lies before the rename.
fn open_on_alpha_body(dir: &TempDir, mode: NavigationMode, view: SidebarView) -> AppState {
    let mut app = AppState::new(dir.path().to_path_buf());
    app.reload_repo_data();
    app.open_file_at_line("src/new.rs", Some(3));
    app.active_panel = ActivePanel::CodeViewer;
    app.set_sidebar_view(view);
    app.set_navigation_mode(mode);
    assert_eq!(app.code_lines[app.cursor_line - 1], "    let a = 2;");
    app
}

#[test]
fn test_line_history_reports_the_old_path() {
    let dir = setup_repo();
    let repo = GitRepo::open(dir.path()).unwrap();

    let commits = repo.get_line_commits("src/new.rs", 3, 3, None).unwrap();

    let seen: Vec<(&str, Option<&str>)> = commits
        .iter()
        .map(|c| (c.summary.as_str(), c.path.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [("Commit 2", Some("old.rs")), ("Commit 1", Some("old.rs"))]
    );
}

#[test]
fn test_line_jump_crosses_rename_and_returns() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Line, SidebarView::CommitTimeline);

    // Back to Commit 2, where the file is still old.rs.
    app.dispatch_action(Action::JumpPrevLine);
    assert!(
        app.status_message.contains("Commit 2"),
        "{}",
        app.status_message
    );
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));
    assert_eq!(app.code_lines, lines(V2));
    assert_eq!(
        app.code_lines[app.cursor_line - 1],
        "    let a = 2;",
        "cursor should follow the line across the rename"
    );
    assert_eq!(app.history_file_path().as_deref(), Some("src/new.rs"));

    // Further back to Commit 1, staying on the old path.
    app.dispatch_action(Action::JumpPrevLine);
    assert!(
        app.status_message.contains("Commit 1"),
        "{}",
        app.status_message
    );
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));
    assert_eq!(app.code_lines, lines(V1));
    assert_eq!(app.code_lines[app.cursor_line - 1], "    let a = 1;");

    // Forward again to Commit 2, then past the newest change back to the working tree.
    app.dispatch_action(Action::JumpNextLine);
    assert_eq!(app.code_lines, lines(V2));
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));

    app.dispatch_action(Action::JumpNextLine);
    assert_eq!(app.selected_commit_hash, None);
    assert_eq!(app.active_file.as_deref(), Some("src/new.rs"));
    assert_eq!(app.code_lines, lines(V4));
    assert_eq!(
        app.code_lines[app.cursor_line - 1],
        "    let a = 2;",
        "cursor should follow the line back across the rename"
    );
    assert_eq!(app.history_file_path().as_deref(), Some("src/new.rs"));
}

#[test]
fn test_function_jump_crosses_rename() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Function, SidebarView::CommitTimeline);

    app.dispatch_action(Action::JumpPrevFunction);

    assert!(
        app.status_message.contains("Commit 2"),
        "{}",
        app.status_message
    );
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));
    assert_eq!(app.code_lines, lines(V2));
}

#[test]
fn test_quit_from_renamed_commit_restores_working_tree_path() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Line, SidebarView::CommitTimeline);
    app.dispatch_action(Action::JumpPrevLine);
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));

    app.dispatch_action(Action::Quit); // leaves time travel, does not exit

    assert!(app.running);
    assert_eq!(app.selected_commit_hash, None);
    assert_eq!(app.active_file.as_deref(), Some("src/new.rs"));
    assert_eq!(app.code_lines, lines(V4));
}

#[test]
fn test_rename_jump_keeps_the_file_in_modified_files_view() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Line, SidebarView::ModifiedFiles);

    // Commit 2 also changes a.txt, which sorts first; the jump must stay on the renamed file.
    app.dispatch_action(Action::JumpPrevLine);

    assert_eq!(app.active_file.as_deref(), Some("old.rs"));
    assert_eq!(app.code_lines, lines(V2));
    assert_eq!(
        app.modified_files[app.modified_selected].path, "old.rs",
        "the renamed file should be the selected entry"
    );
}

#[test]
fn test_rename_jump_keeps_the_file_in_explorer_view() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Line, SidebarView::FileExplorer);

    app.dispatch_action(Action::JumpPrevLine);
    assert_eq!(app.active_file.as_deref(), Some("old.rs"));
    assert_eq!(app.code_lines, lines(V2));
    assert_eq!(app.files, vec!["a.txt", "old.rs"]);

    app.dispatch_action(Action::JumpNextLine);
    assert_eq!(app.selected_commit_hash, None);
    assert_eq!(app.active_file.as_deref(), Some("src/new.rs"));
    assert_eq!(app.code_lines, lines(V4));
}

#[test]
fn test_candidates_stay_stable_while_viewing_the_old_path() {
    let dir = setup_repo();
    let mut app = open_on_alpha_body(&dir, NavigationMode::Line, SidebarView::CommitTimeline);
    let before: Vec<String> = app
        .candidate_commits
        .iter()
        .map(|c| c.message.clone())
        .collect();
    assert_eq!(before, ["Commit 2", "Commit 1"]);

    app.dispatch_action(Action::JumpPrevLine);
    app.move_selection_down();
    app.move_selection_up();

    let after: Vec<String> = app
        .candidate_commits
        .iter()
        .map(|c| c.message.clone())
        .collect();
    assert_eq!(after, before);
    assert_eq!(
        app.candidate_commits[app.candidate_selected].message,
        "Commit 2"
    );
}
