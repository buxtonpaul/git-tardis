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
fn test_line_commits_zero_start_line_and_path_cleaning() {
    let (_dir, repo) = setup_test_repo();

    commit_file(
        &repo,
        "file with spaces.txt",
        "line 1\nline 2\nline 3\n",
        "Initial commit with spaces in filename",
    );
    commit_file(
        &repo,
        "file with spaces.txt",
        "line 1\nline 2 updated\nline 3\n",
        "Update line 2 in filename with spaces",
    );

    // 1. Zero start_line should be clamped to 1 and succeed without Git error
    let zero_line_history = repo
        .get_line_commits("file with spaces.txt", 0, 2, None)
        .expect("Zero start_line should be clamped to 1 and succeed");
    assert_eq!(zero_line_history.len(), 2);

    // 2. Relative prefix "./" and trailing spaces should be cleaned
    let cleaned_path_history = repo
        .get_line_commits("  ./file with spaces.txt  ", 2, 2, None)
        .expect("Path with ./ prefix and whitespace should be cleaned and succeed");
    assert_eq!(cleaned_path_history.len(), 2);
    assert_eq!(
        cleaned_path_history[0].summary,
        "Update line 2 in filename with spaces"
    );
    assert_eq!(
        cleaned_path_history[1].summary,
        "Initial commit with spaces in filename"
    );

    // 3. Empty or whitespace-only paths should return Ok(Vec::new()) without executing git log
    let empty_path_res = repo.get_line_commits("   ", 1, 5, None).unwrap();
    assert!(empty_path_res.is_empty());
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

#[test]
fn test_get_commit_files_special_filenames_and_renames() {
    let (_dir, repo) = setup_test_repo();

    let special_filename = "file with spaces and\ttabs.txt";
    commit_file(&repo, special_filename, "special content", "Commit Special");

    let history = repo.get_commit_history(Some(1)).unwrap();
    let c_hash = &history[0].hash;

    let files = repo.get_commit_files(c_hash).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, special_filename);
    assert_eq!(files[0].index_status, 'A');

    // Test rename operation
    let renamed_filename = "renamed file with spaces.txt";
    let status = Command::new("git")
        .args(["mv", special_filename, renamed_filename])
        .current_dir(repo.work_dir())
        .status()
        .unwrap();
    assert!(status.success());

    let status_commit = Command::new("git")
        .args(["commit", "-m", "Rename special file"])
        .current_dir(repo.work_dir())
        .status()
        .unwrap();
    assert!(status_commit.success());

    let rename_history = repo.get_commit_history(Some(1)).unwrap();
    let rename_hash = &rename_history[0].hash;

    let rename_files = repo.get_commit_files(rename_hash).unwrap();
    assert_eq!(rename_files.len(), 1);
    assert_eq!(rename_files[0].path, renamed_filename);
    assert_eq!(rename_files[0].index_status, 'R');
}

#[test]
fn test_list_files_at_commit() {
    let (_dir, repo) = setup_test_repo();

    commit_file(&repo, "alpha.txt", "alpha content\n", "Add alpha");
    let c1 = repo.get_commit_history(None).unwrap()[0].hash.clone();

    commit_file(&repo, "beta.txt", "beta content\n", "Add beta");
    let c2 = repo.get_commit_history(None).unwrap()[0].hash.clone();

    // c1 has alpha.txt
    let files_c1 = repo
        .list_files_at_commit(&c1)
        .expect("list_files_at_commit c1 failed");
    assert_eq!(files_c1, vec!["alpha.txt"]);

    // c2 has alpha.txt and beta.txt
    let files_c2 = repo
        .list_files_at_commit(&c2)
        .expect("list_files_at_commit c2 failed");
    assert_eq!(files_c2, vec!["alpha.txt", "beta.txt"]);
}

#[test]
fn test_git_dir_resolution() {
    let (_dir, repo) = setup_test_repo();

    let git_dir = repo.git_dir().expect("git_dir resolution failed");
    assert!(git_dir.exists());
    assert!(git_dir.join("HEAD").exists());

    let rebase_git_dir = git_tardis::rebase::get_git_dir(repo.work_dir());
    assert_eq!(git_dir, rebase_git_dir);
}
