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
    ConflictExited(Option<String>),
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
        } else if line.starts_with("merge ") || line.starts_with("m ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let is_match = parts.iter().any(|part| {
                part.len() >= 7
                    && part.chars().all(|c| c.is_ascii_hexdigit())
                    && (part.starts_with(target_hash) || target_hash.starts_with(part))
            });
            if is_match {
                modified.push_str(line);
                modified.push('\n');
                modified.push_str("break\n");
                matched = true;
                continue;
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

/// Resolve the exact Git directory for a repository path
pub fn get_git_dir(repo_path: &Path) -> PathBuf {
    // An ordinary checkout keeps its git directory right here; asking git is only needed
    // for worktrees, submodules, subdirectories or an overridden GIT_DIR.
    let dot_git = repo_path.join(".git");
    if dot_git.is_dir() && std::env::var_os("GIT_DIR").is_none() {
        return dot_git;
    }
    let repo = crate::git::GitRepo::new(repo_path);
    repo.git_dir().unwrap_or_else(|_| repo_path.join(".git"))
}

/// Check if a rebase is currently in progress in the repository
pub fn is_rebase_in_progress(repo_path: &Path) -> bool {
    let git_dir = get_git_dir(repo_path);
    git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists()
}

/// Prompt and handle startup protection if repository is launched while mid-rebase.
/// Returns `Ok(true)` to proceed with launching Git-tardis, or `Ok(false)` to exit.
pub fn check_and_handle_startup_rebase<R: BufRead>(
    repo_path: &Path,
    mut input: R,
) -> Result<bool, Box<dyn std::error::Error>> {
    if is_rebase_in_progress(repo_path) {
        println!("\n⚠️  A Git rebase is currently in progress in this repository.");
        println!("Options:");
        println!("  [1] Exit Git-tardis to finish or resolve rebase in your terminal");
        println!("  [2] Abort active rebase (git rebase --abort) and launch Git-tardis");
        print!("\nEnter choice [1/2]: ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        let _ = input.read_line(&mut choice);

        if choice.trim() == "2" {
            println!("\nAborting active rebase and restoring repository...");
            let status = Command::new("git")
                .current_dir(repo_path)
                .args(["rebase", "--abort"])
                .status()?;
            if !status.success() {
                return Err("Failed to abort active rebase.".into());
            }
            println!("Active rebase aborted successfully. Launching Git-tardis...\n");
            return Ok(true);
        } else {
            println!(
                "\nGit-tardis exited. You can complete the rebase with `git rebase --continue` or abort with `git rebase --abort`."
            );
            return Ok(false);
        }
    }
    Ok(true)
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

    let binary_name = format!("git-tardis{}", env::consts::EXE_SUFFIX);
    if let Some(parent) = current.parent() {
        let target_bin = parent.join(&binary_name);
        if target_bin.exists() {
            return Ok(target_bin);
        }
        if let Some(grandparent) = parent.parent() {
            let target_bin = grandparent.join(&binary_name);
            if target_bin.exists() {
                return Ok(target_bin);
            }
        }
    }

    Ok(current)
}

/// The command git runs as `GIT_SEQUENCE_EDITOR`. Git passes it to a POSIX shell, including
/// on Windows, where backslashes in the path would be read as escapes; forward slashes are
/// accepted there too.
fn sequence_editor_command(exe_path: &str, target_hash: &str) -> String {
    let exe_path = if cfg!(windows) {
        exe_path.replace('\\', "/")
    } else {
        exe_path.to_string()
    };
    format!("\"{}\" --mark-edit {}", exe_path, target_hash)
}

/// The shell to drop the user into while a rebase is paused: `$SHELL` where that is set,
/// otherwise the platform's command interpreter.
fn interactive_shell() -> String {
    if let Ok(shell) = env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }
    if cfg!(windows) {
        env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        "/bin/sh".to_string()
    }
}

/// Execute interactive "Edit Here" rebase targeting a specific commit
pub fn execute_edit_here<R: BufRead>(
    repo_path: &Path,
    target_hash: &str,
    mut input: R,
    in_alternate_screen: &mut bool,
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

    let sequence_editor_cmd = sequence_editor_command(exe_str, target_hash);

    // Suspend TUI mode
    let mut stdout = io::stdout();
    let is_tty = enable_raw_mode().is_ok();
    if is_tty {
        let _ = disable_raw_mode();
        let _ = execute!(stdout, LeaveAlternateScreen, Show);
    }
    *in_alternate_screen = false;

    println!("\n================================================================================");
    println!("Git-tardis: Pausing timeline at commit {}", target_hash);
    println!("================================================================\n");

    let mut rebase_cmd = Command::new("git");
    rebase_cmd.current_dir(repo_path);
    rebase_cmd.env("GIT_SEQUENCE_EDITOR", &sequence_editor_cmd);
    rebase_cmd
        .arg("rebase")
        .arg("-i")
        .arg("--autostash")
        .arg("--rebase-merges");
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
            *in_alternate_screen = true;
            return RebaseResult::Error(format!("Failed to execute git rebase: {}", e));
        }
    };

    let git_dir = get_git_dir(repo_path);
    let rebase_merge_dir = git_dir.join("rebase-merge");

    if rebase_merge_dir.exists() {
        // Stopped at target commit! Spawn interactive subshell unless non-interactive test
        println!(
            "\n================================================================================"
        );
        println!("Git-tardis: Paused at commit {}.", target_hash);
        println!("You are now in an interactive shell.");
        println!("  - Edit files and use 'git commit --amend' to update this commit");
        println!("  - Run 'git rebase --continue' when finished");
        if cfg!(windows) {
            println!("  - Or exit this subshell ('exit') to resume");
        } else {
            println!("  - Or exit this subshell ('exit' or Ctrl-D) to resume");
        }
        println!(
            "================================================================================\n"
        );

        if let Ok(cmd) = env::var("GIT_TARDIS_TEST_CMD") {
            let _ = Command::new("sh")
                .current_dir(repo_path)
                .args(["-c", &cmd])
                .status();
        } else if env::var("GIT_TARDIS_NON_INTERACTIVE").is_err() {
            let _ = Command::new(interactive_shell())
                .current_dir(repo_path)
                .status();
        }

        // After subshell exits, check if rebase is still in progress
        if rebase_merge_dir.exists() {
            // Check for uncommitted working tree changes
            let has_uncommitted = Command::new("git")
                .current_dir(repo_path)
                .args(["status", "--porcelain"])
                .output()
                .map(|out| !out.stdout.iter().all(|b| b.is_ascii_whitespace()))
                .unwrap_or(false);

            if has_uncommitted {
                let msg = "\n⚠️  Uncommitted changes detected in working directory!\n\
                     Git-tardis exited to preserve your uncommitted changes in the active rebase session.\n\
                     To complete the rebase manually:\n  \
                     1. Stage or commit your changes (`git commit --amend` or `git add <files>`)\n  \
                     2. Run `git rebase --continue` (or `git rebase --abort`)\n"
                    .to_string();
                return RebaseResult::ConflictExited(Some(msg));
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
                    let msg = "\nGit-tardis exited. You are in an active rebase session.\n\
                         Fix conflicts with your editor, run `git add <files>`, and `git rebase --continue`."
                        .to_string();
                    return RebaseResult::ConflictExited(Some(msg));
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
                    *in_alternate_screen = true;
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
            let msg = "\nGit-tardis exited. Active rebase session.".to_string();
            return RebaseResult::ConflictExited(Some(msg));
        } else {
            let _ = Command::new("git")
                .current_dir(repo_path)
                .args(["rebase", "--abort"])
                .status();
            if is_tty {
                let _ = enable_raw_mode();
                let _ = execute!(stdout, EnterAlternateScreen, Hide);
            }
            *in_alternate_screen = true;
            return RebaseResult::Aborted;
        }
    }

    // Re-enable TUI
    if is_tty {
        let _ = enable_raw_mode();
        let _ = execute!(stdout, EnterAlternateScreen, Hide);
    }
    *in_alternate_screen = true;

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
