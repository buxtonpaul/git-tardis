use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::env;
use std::fs;
use std::io;
use std::process::{Command, ExitStatus};
use tempfile::TempDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check if invoked as sequence editor helper
    if args.len() >= 4 && args[1] == "--mark-edit" {
        let target_hash = &args[2];
        let todo_path = &args[3];
        return handle_sequence_editor_mark_edit(target_hash, todo_path);
    } else if args.len() >= 2 && args[1] == "--noop" {
        // No-op sequence editor: exits 0 without editing (accepts default todo list)
        return Ok(());
    }

    println!("=== Git-Tardis Interactive Rebase PoC ===");

    // Determine path to this executable so Git can invoke it as GIT_SEQUENCE_EDITOR
    let current_exe = env::current_exe()?;
    let exe_str = current_exe.to_str().ok_or("Invalid UTF-8 in exe path")?;

    println!("Self binary path: {}", exe_str);

    // 1. Run Scenario 1: Inline Rewrite (Autosquash)
    println!("\n--- [Scenario 1] Testing Inline Rewrite (Silent Autosquash) ---");
    test_inline_rewrite(exe_str)?;

    // 2. Run Scenario 1 Conflict handling: Inline Rewrite Conflict
    println!("\n--- [Scenario 1 Conflict] Testing Inline Rewrite Conflict Handling ---");
    test_inline_rewrite_conflict()?;

    // 3. Run Scenario 2: Edit Here (Sequence Editor + TUI Suspend/Resume)
    println!("\n--- [Scenario 2] Testing 'Edit Here' (Sequence Editor & TUI Suspend) ---");
    test_edit_here(exe_str)?;

    println!("\n=== All Interactive Rebase PoC scenarios completed successfully! ===");
    Ok(())
}

/// Helper function invoked when Git calls this binary as `GIT_SEQUENCE_EDITOR`
fn handle_sequence_editor_mark_edit(
    target_hash: &str,
    todo_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(todo_path)?;
    let mut modified = String::new();
    let mut matched = false;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("pick ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let commit_hash = parts[1];
                if commit_hash.starts_with(target_hash) || target_hash.starts_with(commit_hash) {
                    // Replace 'pick' with 'edit'
                    modified.push_str("edit ");
                    modified.push_str(rest);
                    modified.push('\n');
                    matched = true;
                    continue;
                }
            }
        }
        modified.push_str(line);
        modified.push('\n');
    }

    if !matched {
        eprintln!(
            "Sequence editor: Warning - Target commit hash '{}' not found in todo list!",
            target_hash
        );
    } else {
        eprintln!(
            "Sequence editor: Successfully marked commit '{}' as 'edit'.",
            target_hash
        );
    }

    fs::write(todo_path, modified)?;
    Ok(())
}

/// Run git command inside working directory
fn run_git(dir: &std::path::Path, args: &[&str]) -> io::Result<ExitStatus> {
    Command::new("git").current_dir(dir).args(args).status()
}

/// Run git command with custom environment variables
fn run_git_env(
    dir: &std::path::Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> io::Result<ExitStatus> {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.status()
}

/// Capture stdout of git command
fn git_output(dir: &std::path::Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(dir).args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "git command failed: {:?}\nstderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Scenario 1: Inline Rewrite via --fixup and --autosquash
fn test_inline_rewrite(_exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Init repository
    run_git(repo_path, &["init", "-b", "main"])?;
    run_git(repo_path, &["config", "user.name", "Test Runner"])?;
    run_git(repo_path, &["config", "user.email", "test@example.com"])?;

    // Commit A
    fs::write(repo_path.join("file_a.txt"), "Initial A\n")?;
    run_git(repo_path, &["add", "file_a.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit A: Initial setup"])?;
    let hash_a = git_output(repo_path, &["rev-parse", "HEAD"])?;

    // Commit B (Target)
    fs::write(repo_path.join("file_b.txt"), "Line 1 in B\nLine 2 in B\n")?;
    run_git(repo_path, &["add", "file_b.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit B: Feature component"])?;
    let hash_b = git_output(repo_path, &["rev-parse", "HEAD"])?;

    // Commit C
    fs::write(repo_path.join("file_c.txt"), "Initial C\n")?;
    run_git(repo_path, &["add", "file_c.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit C: Final additions"])?;

    println!("Repository created with 3 commits:");
    println!("  A: {}", &hash_a[..7]);
    println!("  B (Target): {}", &hash_b[..7]);

    // Perform inline edit targeting Commit B
    fs::write(
        repo_path.join("file_b.txt"),
        "Line 1 in B (REWRITTEN INLINE)\nLine 2 in B\n",
    )?;
    run_git(repo_path, &["add", "file_b.txt"])?;

    // Create fixup commit for B
    let fixup_arg = format!("--fixup={}", hash_b);
    run_git(repo_path, &["commit", &fixup_arg])?;

    println!("Created fixup commit targeted at B.");

    // Execute non-interactive rebase using GIT_SEQUENCE_EDITOR="true" (or ":" on Unix)
    // and --autosquash --autostash
    let upstream = get_rebase_upstream(repo_path, &hash_b)?;
    println!("Executing `git rebase -i --autosquash --autostash {}` with GIT_SEQUENCE_EDITOR=\"true\"...", upstream);

    let mut rebase_args = vec!["rebase", "-i", "--autosquash", "--autostash"];
    if upstream == "--root" {
        rebase_args.push("--root");
    } else {
        rebase_args.push(&upstream);
    }

    let status = run_git_env(repo_path, &rebase_args, &[("GIT_SEQUENCE_EDITOR", "true")])?;

    if !status.success() {
        return Err("Rebase failed unexpectedly during inline rewrite test".into());
    }

    println!("Rebase succeeded!");

    // Verify commit log
    let log = git_output(repo_path, &["log", "--oneline"])?;
    println!("Updated Git log:\n{}", log);

    // Verify file_b.txt content
    let content_b = fs::read_to_string(repo_path.join("file_b.txt"))?;
    assert!(content_b.contains("REWRITTEN INLINE"));
    println!("Verified file_b.txt content contains inline rewrite!");

    Ok(())
}

/// Determine target upstream for rebase (either commit~1 or --root if commit is root)
fn get_rebase_upstream(
    dir: &std::path::Path,
    commit: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let parent_check = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--verify", "--quiet", &format!("{}~1", commit)])
        .output()?;
    if parent_check.status.success() {
        Ok(format!("{}~1", commit))
    } else {
        Ok("--root".to_string())
    }
}
fn test_inline_rewrite_conflict() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Init repository
    run_git(repo_path, &["init", "-b", "main"])?;
    run_git(repo_path, &["config", "user.name", "Test Runner"])?;
    run_git(repo_path, &["config", "user.email", "test@example.com"])?;

    // Commit A
    fs::write(repo_path.join("conflict.txt"), "Line 1\nLine 2\n")?;
    run_git(repo_path, &["add", "conflict.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit A: Initial"])?;
    let hash_a = git_output(repo_path, &["rev-parse", "HEAD"])?;

    // Commit B: Modify conflict.txt
    fs::write(
        repo_path.join("conflict.txt"),
        "Line 1 modified by B\nLine 2\n",
    )?;
    run_git(repo_path, &["add", "conflict.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit B: Modify Line 1"])?;

    // Fixup targeting A that changes Line 1 to something conflicting with B
    fs::write(
        repo_path.join("conflict.txt"),
        "Line 1 modified by Fixup\nLine 2\n",
    )?;
    run_git(repo_path, &["add", "conflict.txt"])?;
    run_git(repo_path, &["commit", &format!("--fixup={}", hash_a)])?;

    // Attempt rebase
    println!("Executing rebase that causes merge conflict...");
    let upstream = get_rebase_upstream(repo_path, &hash_a)?;
    let mut rebase_args = vec!["rebase", "-i", "--autosquash"];
    if upstream == "--root" {
        rebase_args.push("--root");
    } else {
        rebase_args.push(&upstream);
    }

    let status = run_git_env(repo_path, &rebase_args, &[("GIT_SEQUENCE_EDITOR", "true")])?;

    assert!(!status.success(), "Rebase should fail due to conflict");
    println!("Rebase exited with non-zero status as expected!");

    // Check if rebase in progress
    let rebase_merge_dir = repo_path.join(".git").join("rebase-merge");
    assert!(
        rebase_merge_dir.exists(),
        ".git/rebase-merge directory should exist during conflict"
    );
    println!("Detected active rebase-merge state in .git!");

    // Cleanly abort rebase
    run_git(repo_path, &["rebase", "--abort"])?;
    assert!(
        !rebase_merge_dir.exists(),
        ".git/rebase-merge directory should be removed after abort"
    );
    println!("Successfully aborted conflicting rebase!");

    Ok(())
}

/// Scenario 2: Edit Here (Sequence Editor + TUI Suspend/Resume)
fn test_edit_here(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // Init repository
    run_git(repo_path, &["init", "-b", "main"])?;
    run_git(repo_path, &["config", "user.name", "Test Runner"])?;
    run_git(repo_path, &["config", "user.email", "test@example.com"])?;

    // Commit A
    fs::write(repo_path.join("file_a.txt"), "A\n")?;
    run_git(repo_path, &["add", "file_a.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit A"])?;

    // Commit B (Target for edit)
    fs::write(repo_path.join("file_b.txt"), "B original\n")?;
    run_git(repo_path, &["add", "file_b.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit B"])?;
    let hash_b = git_output(repo_path, &["rev-parse", "HEAD"])?;
    let short_b = &hash_b[..7];

    // Commit C
    fs::write(repo_path.join("file_c.txt"), "C\n")?;
    run_git(repo_path, &["add", "file_c.txt"])?;
    run_git(repo_path, &["commit", "-m", "Commit C"])?;

    println!("Targeting Commit B ({}) for 'Edit here'", short_b);

    // Setup sequence editor command: "<exe_path> --mark-edit <short_b>"
    let sequence_editor_cmd = format!("\"{}\" --mark-edit {}", exe_path, short_b);

    // Simulate TUI state before rebase
    let mut stdout = io::stdout();
    println!("Initialising TUI state (raw mode, alternate screen)...");
    let tty_available = enable_raw_mode().is_ok();
    if tty_available {
        let _ = execute!(stdout, EnterAlternateScreen, Hide);
    } else {
        println!("(Non-TTY environment detected, skipping Crossterm raw mode setup)");
    }

    // We start rebase in background / child process
    // Rebase will call sequence_editor_cmd, mark Commit B as 'edit', and stop at B
    let upstream = get_rebase_upstream(repo_path, &hash_b)?;
    let mut rebase_args = vec!["rebase", "-i"];
    if upstream == "--root" {
        rebase_args.push("--root");
    } else {
        rebase_args.push(&upstream);
    }

    // Suspend TUI before launching rebase process or during pause
    println!("Rebase pausing at target commit. Suspending TUI...");
    if tty_available {
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, Show);
    }

    println!(">>> TUI Suspended successfully! <<<");

    let status = run_git_env(
        repo_path,
        &rebase_args,
        &[("GIT_SEQUENCE_EDITOR", &sequence_editor_cmd)],
    )?;

    // Status will return non-zero or pause state because git stopped at commit B!
    // When git rebase stops for 'edit', git returns exit code 0 or leaves process.
    println!("Git rebase status after stopping at commit: {:?}", status);

    // Verify git stopped at Commit B
    let rebase_merge_dir = repo_path.join(".git").join("rebase-merge");
    if rebase_merge_dir.exists() {
        println!("Git rebase successfully stopped at commit B as expected!");

        // In a real TUI application, at this point we spawn an interactive subshell
        // or user's $SHELL or allow editing.
        println!("Simulating user editing files at commit B...");
        fs::write(
            repo_path.join("file_b.txt"),
            "B modified during 'edit here'\n",
        )?;
        run_git(repo_path, &["add", "file_b.txt"])?;

        // Amend commit or run git rebase --continue
        println!("Running `git commit --amend --no-edit`...");
        run_git(repo_path, &["commit", "--amend", "--no-edit"])?;

        println!("Running `git rebase --continue`...");
        run_git(repo_path, &["rebase", "--continue"])?;
    } else {
        println!("Warning: rebase-merge directory did not exist. Checking status.");
    }

    // Resume TUI state
    println!("Resuming TUI state (raw mode, alternate screen)...");
    if tty_available {
        let _ = enable_raw_mode();
        let _ = execute!(stdout, EnterAlternateScreen, Hide);

        // Clean leave TUI
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, Show);
    }

    // Verify content of file_b.txt after rebase completion
    let content_b = fs::read_to_string(repo_path.join("file_b.txt"))?;
    assert!(content_b.contains("modified during 'edit here'"));
    println!("Verified 'Edit here' successfully modified Commit B!");

    Ok(())
}
