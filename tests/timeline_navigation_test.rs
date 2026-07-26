use git_tardis::app::{AppState, NavigationMode};
use git_tardis::git::GitRepo;
use git_tardis::timeline::{JumpDirection, JumpScope, TimelineNavigator};
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

    // Jump PREV -> C3 ("v3")
    app.dispatch_action(Action::JumpPrevFile);
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v3\"); }"]);
    assert!(app.status_message.contains("Jumped [PREVIOUS] (FILE)"));

    // Jump PREV -> C2 ("v2")
    app.dispatch_action(Action::JumpPrevFile);
    assert_eq!(app.code_lines, vec!["fn main() { println!(\"v2\"); }"]);

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

    // Step PREV in Function mode -> should hit C4 (target v3)
    app.dispatch_action(Action::JumpPrevFunction);
    assert!(app.status_message.contains("C4: Modify target to v3"));

    // Step PREV in Function mode -> should hit C3 (target v2)
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

    // Cursor on line 1 -> jumps should only see C2 and C1
    app.cursor_line = 1;

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C2: Modify line 1"));

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C1: Initial data"));

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("Already at oldest commit"));

    // Now test line 3 on working copy
    app.selected_commit_hash = None;
    app.cursor_line = 3;

    app.dispatch_action(Action::JumpPrevLine);
    assert!(app.status_message.contains("C3: Modify line 3"));

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

    // Default mode: File mode
    assert_eq!(app.nav_mode, NavigationMode::File);
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
        .jump(
            repo.work_dir(),
            "foo.txt",
            &["v2".to_string()],
            1,
            None,
            JumpScope::File,
            JumpDirection::Previous,
        )
        .unwrap()
        .unwrap();

    assert_eq!(res.code_lines, vec!["v2"]);
    assert_eq!(res.commit_summary, "Commit 2");
}
