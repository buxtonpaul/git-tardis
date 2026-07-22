pub mod blame;
pub mod diff;
pub mod log;
pub mod status;
pub mod types;

pub use blame::*;
pub use log::*;
pub use types::*;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GitRepo {
    work_dir: PathBuf,
}

impl GitRepo {
    /// Create a new GitRepo instance for the given directory path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            work_dir: path.as_ref().to_path_buf(),
        }
    }

    /// Open a directory and verify that it is a valid Git repository.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GitError> {
        let repo = Self::new(path);
        repo.check_is_repo()?;
        Ok(repo)
    }

    /// Get the working directory path.
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Check if the directory is inside a valid Git repository.
    pub fn check_is_repo(&self) -> Result<(), GitError> {
        let output = self.run_git(&["rev-parse", "--is-inside-work-tree"])?;
        if output.trim() == "true" {
            Ok(())
        } else {
            Err(GitError::NotARepository(
                self.work_dir.display().to_string(),
            ))
        }
    }

    /// Execute a raw `git` command in the working directory and return stdout as a String.
    pub(crate) fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let bytes = self.run_git_bytes(args)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Execute a raw `git` command in the working directory and return stdout bytes.
    pub(crate) fn run_git_bytes(&self, args: &[&str]) -> Result<Vec<u8>, GitError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.work_dir)
            .args(args)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("not a git repository") {
                return Err(GitError::NotARepository(
                    self.work_dir.display().to_string(),
                ));
            }
            return Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                exit_code: output.status.code(),
                stderr,
            });
        }

        Ok(output.stdout)
    }
}
