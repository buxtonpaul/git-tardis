use std::fs;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::git::{GitError, GitRepo};

fn setup_test_repo() -> (TempDir, GitRepo) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Init git repo
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
fn test_format_blame_annotation() {
    use git_tardis::git::{format_blame_annotation, format_relative_time, BlameLine};

    let uncommitted = BlameLine {
        commit_hash: "0000000000000000000000000000000000000000".to_string(),
        orig_line: 1,
        final_line: 1,
        author: "Paul".to_string(),
        author_mail: "paul@example.com".to_string(),
        author_time: 0,
        summary: "".to_string(),
        content: "test".to_string(),
    };
    assert_eq!(
        format_blame_annotation(&uncommitted),
        "Paul • Not Committed Yet"
    );

    let committed = BlameLine {
        commit_hash: "a1b2c3d4e5f6789".to_string(),
        orig_line: 1,
        final_line: 1,
        author: "Alice".to_string(),
        author_mail: "alice@example.com".to_string(),
        author_time: 1600000000,
        summary: "fix bug".to_string(),
        content: "test".to_string(),
    };
    let annotation = format_blame_annotation(&committed);
    assert!(annotation.contains("Alice"));
    assert!(annotation.contains("a1b2c3d"));
    assert!(annotation.contains("fix bug"));

    let time_str = format_relative_time(0);
    assert_eq!(time_str, "");
}

#[test]
fn test_not_a_repository() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let res = GitRepo::open(temp_dir.path());
    assert!(matches!(res, Err(GitError::NotARepository(_))));
}

#[test]
fn test_list_files_and_status() {
    let (_dir, repo) = setup_test_repo();

    // 1. Initial commit
    commit_file(&repo, "file1.txt", "hello world\n", "Initial commit");

    // 2. Add modified file
    fs::write(repo.work_dir().join("file1.txt"), "hello world modified\n").unwrap();

    // 3. Add untracked file
    fs::write(repo.work_dir().join("untracked.txt"), "untracked\n").unwrap();

    // Check list_files
    let files = repo.list_files().expect("Failed to list files");
    assert_eq!(files, vec!["file1.txt", "untracked.txt"]);

    // Check status
    let status = repo.get_status().expect("Failed to get status");
    assert_eq!(status.len(), 2);

    let file1_status = status.iter().find(|s| s.path == "file1.txt").unwrap();
    assert!(file1_status.is_modified());

    let untracked_status = status.iter().find(|s| s.path == "untracked.txt").unwrap();
    assert!(untracked_status.is_untracked());
}

#[test]
fn test_commit_history_and_file_commits() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "a.txt", "line 1\n", "Commit A: Add a.txt");
    commit_file(&repo, "b.txt", "line 1\n", "Commit B: Add b.txt");
    commit_file(&repo, "a.txt", "line 1\nline 2\n", "Commit C: Update a.txt");

    let history = repo.get_commit_history(None).expect("Failed to get log");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].summary, "Commit C: Update a.txt");
    assert_eq!(history[1].summary, "Commit B: Add b.txt");
    assert_eq!(history[2].summary, "Commit A: Add a.txt");
    assert_eq!(history[0].author, "Test User");
    assert_eq!(history[0].email, "test@example.com");

    let a_history = repo
        .get_file_commits("a.txt", None)
        .expect("Failed to get log for a.txt");
    assert_eq!(a_history.len(), 2);
    assert_eq!(a_history[0].summary, "Commit C: Update a.txt");
    assert_eq!(a_history[1].summary, "Commit A: Add a.txt");

    let max_one = repo
        .get_commit_history(Some(1))
        .expect("Failed to get limited log");
    assert_eq!(max_one.len(), 1);
    assert_eq!(max_one[0].summary, "Commit C: Update a.txt");
}

#[test]
fn test_line_commits() {
    let (_dir, repo) = setup_test_repo();

    commit_file(
        &repo,
        "lines.txt",
        "line 1\nline 2\nline 3\nline 4\n",
        "Commit 1: Base lines",
    );
    commit_file(
        &repo,
        "lines.txt",
        "line 1\nline 2 modified\nline 3\nline 4\n",
        "Commit 2: Modify line 2",
    );
    commit_file(
        &repo,
        "lines.txt",
        "line 1\nline 2 modified\nline 3\nline 4 modified\n",
        "Commit 3: Modify line 4",
    );

    let line2_history = repo
        .get_line_commits("lines.txt", 2, 2, None)
        .expect("Failed to get line log");
    assert_eq!(line2_history.len(), 2);
    assert_eq!(line2_history[0].summary, "Commit 2: Modify line 2");
    assert_eq!(line2_history[1].summary, "Commit 1: Base lines");
}

#[test]
fn test_diff_and_file_at_commit() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "doc.txt", "v1 content\n", "Commit 1");
    let history1 = repo.get_commit_history(None).unwrap();
    let c1_hash = &history1[0].hash;

    commit_file(&repo, "doc.txt", "v2 content\n", "Commit 2");
    let history2 = repo.get_commit_history(None).unwrap();
    let c2_hash = &history2[0].hash;

    // File content at commit
    let v1_content = repo
        .get_file_at_commit(c1_hash, "doc.txt")
        .expect("Failed to get file at c1");
    assert_eq!(v1_content, "v1 content\n");

    let v2_content = repo
        .get_file_at_commit(c2_hash, "doc.txt")
        .expect("Failed to get file at c2");
    assert_eq!(v2_content, "v2 content\n");

    // Commit diff
    let diff = repo
        .get_diff_commit(c2_hash)
        .expect("Failed to get commit diff");
    assert!(diff.contains("-v1 content"));
    assert!(diff.contains("+v2 content"));

    // Working tree diff
    fs::write(repo.work_dir().join("doc.txt"), "v3 working content\n").unwrap();
    let work_diff = repo
        .get_working_diff(Some("doc.txt"))
        .expect("Failed to get working diff");
    assert!(work_diff.contains("-v2 content"));
    assert!(work_diff.contains("+v3 working content"));
}

#[test]
fn test_blame_queries() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "code.py", "def foo():\n    pass\n", "Add foo");
    commit_file(
        &repo,
        "code.py",
        "def foo():\n    return 42\n",
        "Implement foo",
    );

    let blame = repo
        .get_blame("code.py", None, None)
        .expect("Failed to get blame");
    assert_eq!(blame.len(), 2);
    assert_eq!(blame[0].content, "def foo():");
    assert_eq!(blame[0].summary, "Add foo");
    assert_eq!(blame[0].author, "Test User");

    assert_eq!(blame[1].content, "    return 42");
    assert_eq!(blame[1].summary, "Implement foo");

    // Aggregated hunks
    let hunks = repo
        .get_blame_hunks("code.py", None, None)
        .expect("Failed to get blame hunks");
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0].line_count, 1);
    assert_eq!(hunks[1].line_count, 1);
}

#[test]
fn test_get_commit_files() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "file_a.txt", "File A content", "Commit A");
    commit_file(&repo, "file_b.txt", "File B content", "Commit B");

    let history = repo.get_commit_history(Some(10)).unwrap();
    assert!(history.len() >= 2);

    let commit_b_hash = &history[0].hash; // latest commit (Commit B)
    let files_b = repo.get_commit_files(commit_b_hash).unwrap();
    assert_eq!(files_b.len(), 1);
    assert_eq!(files_b[0].path, "file_b.txt");

    let commit_a_hash = &history[1].hash; // earlier commit (Commit A)
    let files_a = repo.get_commit_files(commit_a_hash).unwrap();
    assert_eq!(files_a.len(), 1);
    assert_eq!(files_a[0].path, "file_a.txt");
}
