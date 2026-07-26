use std::fs;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{ActivePanel, AppState};
use git_tardis::git::{format_blame_annotation, GitRepo};

fn setup_multi_commit_repo() -> (TempDir, GitRepo) {
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
    run(&["config", "user.name", "Alice Author"]);
    run(&["config", "user.email", "alice@example.com"]);

    // Commit 1: Lines 1-5 by Alice.
    let lines_c1 = (1..=5)
        .map(|i| format!("Line {} content", i))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(repo_path.join("file.py"), &lines_c1).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Initial commit by Alice"]);

    // Commit 2: Append Lines 6-12 by Bob
    run(&["config", "user.name", "Bob Builder"]);
    run(&["config", "user.email", "bob@example.com"]);
    let lines_c2 = lines_c1
        + &(6..=12)
            .map(|i| format!("Line {} content", i))
            .collect::<Vec<_>>()
            .join("\n")
        + "\n";
    fs::write(repo_path.join("file.py"), &lines_c2).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Add lines 6-12 by Bob"]);

    // Commit 3: Append Lines 13-20 by Charlie
    run(&["config", "user.name", "Charlie Coder"]);
    run(&["config", "user.email", "charlie@example.com"]);
    let lines_c3 = lines_c2
        + &(13..=20)
            .map(|i| format!("Line {} content", i))
            .collect::<Vec<_>>()
            .join("\n")
        + "\n";
    fs::write(repo_path.join("file.py"), &lines_c3).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Add lines 13-20 by Charlie"]);

    let repo = GitRepo::open(repo_path).expect("Failed to open git repo");
    (temp_dir, repo)
}

#[test]
fn test_blame_caching_zero_subprocesses_during_cursor_navigation() {
    let (_temp, repo) = setup_multi_commit_repo();
    let mut app = AppState::new(repo.work_dir().to_path_buf());

    app.files = vec!["file.py".to_string()];
    app.file_selected = 0;
    app.load_currently_selected_file();
    app.active_panel = ActivePanel::CodeViewer;

    assert_eq!(app.code_lines.len(), 20);
    assert_eq!(app.cursor_line, 1);

    // Initial load: 1 subprocess spawned to cache whole-file blame
    assert_eq!(app.blame_subprocess_count, 1);
    assert!(app.current_line_blame.is_some());

    // Navigate cursor down across all 20 lines
    for line in 1..=20 {
        assert_eq!(app.cursor_line, line);
        assert!(
            app.current_line_blame.is_some(),
            "Expected blame for line {}",
            line
        );

        // Subprocess count MUST remain at 1 throughout cursor line navigation
        assert_eq!(
            app.blame_subprocess_count, 1,
            "Subprocess spawned during cursor movement to line {}",
            line
        );

        if line < 20 {
            app.move_selection_down();
        }
    }

    // Navigate cursor back up across all 20 lines
    for line in (1..=20).rev() {
        assert_eq!(app.cursor_line, line);
        assert!(
            app.current_line_blame.is_some(),
            "Expected blame for line {}",
            line
        );

        // Subprocess count MUST remain at 1
        assert_eq!(
            app.blame_subprocess_count, 1,
            "Subprocess spawned during reverse cursor movement to line {}",
            line
        );

        if line > 1 {
            app.move_selection_up();
        }
    }
}

#[test]
fn test_blame_cache_annotations_match_baseline_single_line() {
    let (_temp, repo) = setup_multi_commit_repo();
    let mut app = AppState::new(repo.work_dir().to_path_buf());

    app.files = vec!["file.py".to_string()];
    app.file_selected = 0;
    app.load_currently_selected_file();
    app.active_panel = ActivePanel::CodeViewer;

    assert_eq!(app.code_lines.len(), 20);

    // For every line in the file, verify cached blame matches single-line git blame -p -L line,line
    for line in 1..=20 {
        app.cursor_line = line;
        app.update_current_line_blame();

        let cached_blame = app
            .current_line_blame
            .as_ref()
            .expect("Cached blame missing");

        // Baseline: run git blame -p -L line,line directly
        let baseline_blame_vec = repo
            .get_blame_at_commit(None, "file.py", Some(line), Some(line))
            .expect("Baseline single-line blame failed");
        let baseline_blame = baseline_blame_vec.first().expect("Baseline blame empty");

        // Assert all blame fields match baseline exactly
        assert_eq!(
            cached_blame.commit_hash, baseline_blame.commit_hash,
            "Mismatch in commit_hash for line {}",
            line
        );
        assert_eq!(
            cached_blame.author, baseline_blame.author,
            "Mismatch in author for line {}",
            line
        );
        assert_eq!(
            cached_blame.author_mail, baseline_blame.author_mail,
            "Mismatch in author_mail for line {}",
            line
        );
        assert_eq!(
            cached_blame.author_time, baseline_blame.author_time,
            "Mismatch in author_time for line {}",
            line
        );
        assert_eq!(
            cached_blame.summary, baseline_blame.summary,
            "Mismatch in summary for line {}",
            line
        );
        assert_eq!(
            cached_blame.orig_line, baseline_blame.orig_line,
            "Mismatch in orig_line for line {}",
            line
        );
        assert_eq!(
            cached_blame.final_line, baseline_blame.final_line,
            "Mismatch in final_line for line {}",
            line
        );

        // Assert formatted annotations match baseline exactly
        let cached_annotation = format_blame_annotation(cached_blame);
        let baseline_annotation = format_blame_annotation(baseline_blame);
        assert_eq!(
            cached_annotation, baseline_annotation,
            "Mismatch in formatted annotation string for line {}",
            line
        );
    }
}

#[test]
fn test_blame_cache_invalidation_and_historical_commit_blame() {
    let (_temp, repo) = setup_multi_commit_repo();
    let mut app = AppState::new(repo.work_dir().to_path_buf());

    // 1. Load working file
    app.files = vec!["file.py".to_string()];
    app.file_selected = 0;
    app.load_currently_selected_file();
    assert_eq!(app.blame_subprocess_count, 1);

    // Get latest commit hash from commits list
    if let Ok(commits) = repo.get_commit_history(None) {
        assert_eq!(commits.len(), 3);
        let first_commit_hash = &commits[2].hash; // Alice's commit

        // Time travel to first commit
        app.selected_commit_hash = Some(first_commit_hash.clone());
        app.load_currently_selected_file();

        // Separate cache key (file.py, Some(hash)) -> 1 new subprocess on load
        assert_eq!(app.blame_subprocess_count, 2);

        // Verify line 1 blame at historical commit
        app.cursor_line = 1;
        app.update_current_line_blame();
        let hist_blame = app.current_line_blame.as_ref().unwrap();
        assert_eq!(hist_blame.author, "Alice Author");
        assert_eq!(hist_blame.summary, "Initial commit by Alice");

        // Cursor navigation at historical commit uses cache without new subprocesses
        app.cursor_line = 3;
        app.update_current_line_blame();
        assert_eq!(app.blame_subprocess_count, 2);

        // Return from time travel resets commit hash and reloads file
        app.reset_time_travel();
        assert_eq!(app.selected_commit_hash, None);

        // load_currently_selected_file re-populates working copy cache key
        assert_eq!(app.blame_subprocess_count, 3);
        assert_eq!(app.blame_cache.len(), 1);

        // Subsequent update uses cache without new subprocesses
        app.update_current_line_blame();
        assert_eq!(app.blame_subprocess_count, 3);
    }
}
