use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum RebaseResult {
    Completed,
    ConflictExited,
    Aborted,
    Error(String),
}

/// Helper function invoked when Git calls this binary as `GIT_SEQUENCE_EDITOR`
/// with `--mark-edit <target_hash> <todo_file_path>`.
pub fn handle_sequence_editor_mark_edit(
    target_hash: &str,
    todo_file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(todo_file_path)?;
    let mut modified = String::new();
    let mut matched = false;

    for line in content.lines() {
        if line.starts_with("pick ") || line.starts_with("p ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let commit_hash = parts[1];
                if commit_hash.starts_with(target_hash) || target_hash.starts_with(commit_hash) {
                    let rest = &line[parts[0].len()..];
                    modified.push_str("edit");
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
            "Sequence editor warning: Target commit hash '{}' not found in todo list.",
            target_hash
        );
    }

    fs::write(todo_file_path, modified)?;
    Ok(())
}

/// Determine target upstream for rebase (either commit~1 or --root if commit is root)
pub fn get_rebase_upstream(dir: &Path, commit_hash: &str) -> Result<String, String> {
    let parent_check = Command::new("git")
        .current_dir(dir)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{}~1", commit_hash),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if parent_check.status.success() {
        Ok(format!("{}~1", commit_hash))
    } else {
        Ok("--root".to_string())
    }
}

/// Check if a rebase is currently in progress in the repository
pub fn is_rebase_in_progress(repo_path: &Path) -> bool {
    repo_path.join(".git").join("rebase-merge").exists()
        || repo_path.join(".git").join("rebase-apply").exists()
}

/// Locate the `git-tardis` binary for `GIT_SEQUENCE_EDITOR` invocations
pub fn find_git_tardis_binary() -> Result<PathBuf, String> {
    if let Ok(override_path) = env::var("GIT_TARDIS_BIN_PATH") {
        let p = PathBuf::from(override_path);
        if p.exists() {
            return Ok(p);
        }
    }

    if let Ok(cargo_bin) = env::var("CARGO_BIN_EXE_git-tardis") {
        let p = PathBuf::from(cargo_bin);
        if p.exists() {
            return Ok(p);
        }
    }

    let current =
        env::current_exe().map_err(|e| format!("Failed to locate current executable: {}", e))?;
    let file_name = current.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if file_name == "git-tardis" || file_name == "git-tardis.exe" {
        return Ok(current);
    }

    if let Some(parent) = current.parent() {
        let target_bin = parent.join("git-tardis");
        if target_bin.exists() {
            return Ok(target_bin);
        }
        if let Some(grandparent) = parent.parent() {
            let target_bin = grandparent.join("git-tardis");
            if target_bin.exists() {
                return Ok(target_bin);
            }
        }
    }

    Ok(current)
}

/// Execute interactive "Edit Here" rebase targeting a specific commit
pub fn execute_edit_here<R: BufRead>(
    repo_path: &Path,
    target_hash: &str,
    mut input: R,
) -> RebaseResult {
    if is_rebase_in_progress(repo_path) {
        return RebaseResult::Error(
            "A rebase is already in progress in this repository.".to_string(),
        );
    }

    let tardis_bin = match find_git_tardis_binary() {
        Ok(bin) => bin,
        Err(e) => return RebaseResult::Error(e),
    };
    let exe_str = match tardis_bin.to_str() {
        Some(s) => s,
        None => return RebaseResult::Error("Invalid UTF-8 in executable path".to_string()),
    };

    let upstream = match get_rebase_upstream(repo_path, target_hash) {
        Ok(u) => u,
        Err(e) => return RebaseResult::Error(e),
    };

    let sequence_editor_cmd = format!("\"{}\" --mark-edit {}", exe_str, target_hash);

    // Suspend TUI mode
    let mut stdout = io::stdout();
    let is_tty = enable_raw_mode().is_ok();
    if is_tty {
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, Show);
    }

    println!("\n================================================================================");
    println!("Git-tardis: Pausing timeline at commit {}", target_hash);
    println!("================================================================ clever\n");

    let mut rebase_cmd = Command::new("git");
    rebase_cmd.current_dir(repo_path);
    rebase_cmd.env("GIT_SEQUENCE_EDITOR", &sequence_editor_cmd);
    rebase_cmd.arg("rebase").arg("-i").arg("--autostash");
    if upstream == "--root" {
        rebase_cmd.arg("--root");
    } else {
        rebase_cmd.arg(&upstream);
    }

    let status = match rebase_cmd.status() {
        Ok(s) => s,
        Err(e) => {
            if is_tty {
                let _ = enable_raw_mode();
                let _ = execute!(stdout, EnterAlternateScreen, Hide);
            }
            return RebaseResult::Error(format!("Failed to execute git rebase: {}", e));
        }
    };

    let rebase_merge_dir = repo_path.join(".git").join("rebase-merge");

    if rebase_merge_dir.exists() {
        // Stopped at target commit! Spawn interactive subshell unless non-interactive test
        println!(
            "\n================================================================================"
        );
        println!("Git-tardis: Paused at commit {}.", target_hash);
        println!("You are now in an interactive shell.");
        println!("  - Edit files and use 'git commit --amend' to update this commit");
        println!("  - Run 'git rebase --continue' when finished");
        println!("  - Or exit this subshell ('exit' or Ctrl-D) to resume");
        println!(
            "================================================================================\n"
        );

        if let Ok(cmd) = env::var("GIT_TARDIS_TEST_CMD") {
            let _ = Command::new("sh")
                .current_dir(repo_path)
                .args(["-c", &cmd])
                .status();
        } else if env::var("GIT_TARDIS_NON_INTERACTIVE").is_err() {
            let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let _ = Command::new(&shell).current_dir(repo_path).status();
        }

        // After subshell exits, check if rebase is still in progress
        if rebase_merge_dir.exists() {
            // Auto-stage and amend any uncommitted file changes made during the edit session
            let has_uncommitted = Command::new("git")
                .current_dir(repo_path)
                .args(["status", "--porcelain"])
                .output()
                .map(|out| !out.stdout.iter().all(|b| b.is_ascii_whitespace()))
                .unwrap_or(false);

            if has_uncommitted {
                println!(
                    "\nAuto-staging and amending changes into commit {}...",
                    &target_hash[..7.min(target_hash.len())]
                );
                let _ = Command::new("git")
                    .current_dir(repo_path)
                    .args(["add", "-A"])
                    .status();
                let _ = Command::new("git")
                    .current_dir(repo_path)
                    .args(["commit", "--amend", "--no-edit"])
                    .status();
            }

            println!("\nResuming rebase (`git rebase --continue`)...");
            let continue_status = Command::new("git")
                .current_dir(repo_path)
                .args(["rebase", "--continue"])
                .status();

            if continue_status.is_ok_and(|s| !s.success()) || rebase_merge_dir.exists() {
                // Conflict encountered during continuation!
                println!("\n⚠️  Merge conflict detected during rebase!");
                println!("How would you like to proceed?");
                println!("  [1] Exit Git-tardis to resolve conflicts in your terminal");
                println!("  [2] Abort rebase (git rebase --abort) and return to Git-tardis TUI");
                print!("\nEnter choice [1/2]: ");
                let _ = io::stdout().flush();

                let mut choice = String::new();
                let _ = input.read_line(&mut choice);

                if choice.trim() == "1" {
                    println!("\nGit-tardis exited. You are in an active rebase session.");
                    println!("Fix conflicts with your editor, run `git add <files>`, and `git rebase --continue`.");
                    return RebaseResult::ConflictExited;
                } else {
                    println!("\nAborting rebase and restoring pre-rebase state...");
                    let _ = Command::new("git")
                        .current_dir(repo_path)
                        .args(["rebase", "--abort"])
                        .status();
                    if is_tty {
                        let _ = enable_raw_mode();
                        let _ = execute!(stdout, EnterAlternateScreen, Hide);
                    }
                    return RebaseResult::Aborted;
                }
            }
        }
    } else if !status.success() {
        // Initial rebase invocation failed
        println!("\n⚠️  Rebase failed to initialize or encountered a conflict.");
        println!("How would you like to proceed?");
        println!("  [1] Exit Git-tardis to resolve in your terminal");
        println!("  [2] Abort rebase (git rebase --abort) and return to Git-tardis TUI");
        print!("\nEnter choice [1/2]: ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        let _ = input.read_line(&mut choice);

        if choice.trim() == "1" {
            println!("\nGit-tardis exited. Active rebase session.");
            return RebaseResult::ConflictExited;
        } else {
            let _ = Command::new("git")
                .current_dir(repo_path)
                .args(["rebase", "--abort"])
                .status();
            if is_tty {
                let _ = enable_raw_mode();
                let _ = execute!(stdout, EnterAlternateScreen, Hide);
            }
            return RebaseResult::Aborted;
        }
    }

    // Re-enable TUI
    if is_tty {
        let _ = enable_raw_mode();
        let _ = execute!(stdout, EnterAlternateScreen, Hide);
    }

    RebaseResult::Completed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_handle_sequence_editor_mark_edit() {
        let temp_dir = TempDir::new().unwrap();
        let todo_path = temp_dir.path().join("git-rebase-todo");

        let initial_content = "pick a1b2c3d Commit 1\npick e5f6g7h Commit 2\n";
        fs::write(&todo_path, initial_content).unwrap();

        handle_sequence_editor_mark_edit("e5f6g7h", &todo_path).unwrap();

        let updated_content = fs::read_to_string(&todo_path).unwrap();
        assert!(updated_content.contains("pick a1b2c3d Commit 1"));
        assert!(updated_content.contains("edit e5f6g7h Commit 2"));
    }
}
