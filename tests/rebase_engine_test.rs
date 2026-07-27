use std::fs;
use std::io::Cursor;
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

static REBASE_TEST_MUTEX: Mutex<()> = Mutex::new(());

use git_tardis::rebase::{
    execute_edit_here, get_rebase_upstream, handle_sequence_editor_mark_edit,
    is_rebase_in_progress, RebaseResult,
};

fn setup_test_repo() -> (TempDir, String, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Git command failed: git {}",
            args.join(" ")
        );
    };

    run(&["init"]);
    run(&["config", "user.name", "Rebase Test"]);
    run(&["config", "user.email", "rebase@example.com"]);

    // Commit 1 (Root)
    fs::write(repo_path.join("file_a.txt"), "Initial A\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 1: Root"]);
    let h1 = String::from_utf8(
        Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    // Commit 2 (Target)
    fs::write(repo_path.join("file_b.txt"), "Original B\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 2: Feature B"]);
    let h2 = String::from_utf8(
        Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    // Commit 3
    fs::write(repo_path.join("file_c.txt"), "Initial C\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 3: Feature C"]);
    let h3 = String::from_utf8(
        Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    (temp_dir, h1, h2, h3)
}

#[test]
fn test_get_rebase_upstream_resolution() {
    let (_dir, h1, h2, _h3) = setup_test_repo();
    let repo_path = _dir.path();

    // Root commit upstream should be --root
    let root_upstream = get_rebase_upstream(repo_path, &h1).unwrap();
    assert_eq!(root_upstream, "--root");

    // Non-root commit upstream should be h2~1 (h1)
    let non_root_upstream = get_rebase_upstream(repo_path, &h2).unwrap();
    assert_eq!(non_root_upstream, format!("{}~1", h2));
}

#[test]
fn test_sequence_editor_todo_rewrite() {
    let temp_dir = TempDir::new().unwrap();
    let todo_file = temp_dir.path().join("git-rebase-todo");

    let todo_content = "pick a1b2c3d Commit 1\npick 1234567 Target commit\npick 9876543 Commit 3\n";
    fs::write(&todo_file, todo_content).unwrap();

    handle_sequence_editor_mark_edit("1234567", &todo_file).unwrap();

    let updated = fs::read_to_string(&todo_file).unwrap();
    assert!(updated.contains("pick a1b2c3d Commit 1"));
    assert!(updated.contains("edit 1234567 Target commit"));
    assert!(updated.contains("pick 9876543 Commit 3"));
}

#[test]
fn test_execute_edit_here_successful_rebase() {
    let _lock = REBASE_TEST_MUTEX.lock().unwrap();
    let (temp_dir, _h1, h2, _h3) = setup_test_repo();
    let repo_path = temp_dir.path();

    std::env::set_var(
        "GIT_TARDIS_TEST_CMD",
        "echo 'Modified B during rebase' > file_b.txt && git add file_b.txt && git commit --amend --no-edit",
    );

    let input = Cursor::new("1\n");
    let res = execute_edit_here(repo_path, &h2, input);

    assert_eq!(res, RebaseResult::Completed);
    assert!(!is_rebase_in_progress(repo_path));

    // Verify file_b.txt content after rebase completion
    let content_b = fs::read_to_string(repo_path.join("file_b.txt")).unwrap();
    assert_eq!(content_b, "Modified B during rebase\n");

    std::env::remove_var("GIT_TARDIS_TEST_CMD");
}

#[test]
fn test_execute_edit_here_conflict_and_exit_choice() {
    let _lock = REBASE_TEST_MUTEX.lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Conflict Test"]);
    run(&["config", "user.email", "conflict@example.com"]);

    // Commit 1: file.txt = "Line 1\nLine 2\nLine 3\n"
    fs::write(repo_path.join("file.txt"), "Line 1\nLine 2\nLine 3\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C1"]);
    let h1 = String::from_utf8(
        Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    // Commit 2: Modify Line 2 in file.txt
    fs::write(
        repo_path.join("file.txt"),
        "Line 1\nC2 modification on line 2\nLine 3\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C2"]);

    // Set script command that modifies line 2 in C1 in a conflicting way
    std::env::set_var(
        "GIT_TARDIS_TEST_CMD",
        "printf 'Line 1\\nEditHere modification on line 2\\nLine 3\\n' > file.txt && git add file.txt && git commit --amend --no-edit",
    );

    // Choice 1: Exit Git-tardis to resolve in terminal
    let input = Cursor::new("1\n");
    let res = execute_edit_here(repo_path, &h1, input);

    assert_eq!(res, RebaseResult::ConflictExited);
    assert!(
        is_rebase_in_progress(repo_path),
        "Rebase should remain active for conflict resolution"
    );

    // Clean up rebase
    let _ = Command::new("git")
        .current_dir(repo_path)
        .args(["rebase", "--abort"])
        .status();

    std::env::remove_var("GIT_TARDIS_TEST_CMD");
}

#[test]
fn test_execute_edit_here_conflict_and_abort_choice() {
    let _lock = REBASE_TEST_MUTEX.lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Abort Test"]);
    run(&["config", "user.email", "abort@example.com"]);

    // Commit 1: file.txt = "Line 1\nLine 2\nLine 3\n"
    fs::write(repo_path.join("file.txt"), "Line 1\nLine 2\nLine 3\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C1"]);
    let h1 = String::from_utf8(
        Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    // Commit 2: Modify Line 2 in file.txt
    fs::write(
        repo_path.join("file.txt"),
        "Line 1\nC2 modification on line 2\nLine 3\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "C2"]);

    // Set script command causing conflict
    std::env::set_var(
        "GIT_TARDIS_TEST_CMD",
        "printf 'Line 1\\nEditHere modification on line 2\\nLine 3\\n' > file.txt && git add file.txt && git commit --amend --no-edit",
    );

    // Choice 2: Abort rebase
    let input = Cursor::new("2\n");
    let res = execute_edit_here(repo_path, &h1, input);

    assert_eq!(res, RebaseResult::Aborted);
    assert!(
        !is_rebase_in_progress(repo_path),
        "Rebase should be aborted and state restored"
    );

    std::env::remove_var("GIT_TARDIS_TEST_CMD");
}
