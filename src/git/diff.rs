use super::{GitError, GitRepo};

impl GitRepo {
    /// Retrieve full patch diff for a commit (`git show <commit_hash>`).
    pub fn get_diff_commit(&self, commit_hash: &str) -> Result<String, GitError> {
        self.run_git(&["show", commit_hash])
    }

    /// Retrieve patch diff for a specific file within a commit (`git show <commit_hash> -- <path>`).
    pub fn get_diff_file(&self, commit_hash: &str, path: &str) -> Result<String, GitError> {
        self.run_git(&["show", commit_hash, "--", path])
    }

    /// Retrieve working tree diff relative to HEAD (`git diff HEAD`), including untracked files if needed.
    pub fn get_working_diff(&self, path: Option<&str>) -> Result<String, GitError> {
        let mut args = vec!["diff", "HEAD"];
        if let Some(p) = path {
            args.push("--");
            args.push(p);
        }

        let mut diff_output = match self.run_git(&args) {
            Ok(output) => output,
            Err(_) => {
                // Fallback for initial commit state where HEAD might not resolve yet
                let mut fallback_args = vec!["diff"];
                if let Some(p) = path {
                    fallback_args.push("--");
                    fallback_args.push(p);
                }
                self.run_git(&fallback_args).unwrap_or_default()
            }
        };

        if let Some(p) = path {
            let file_path = self.work_dir.join(p);
            if file_path.is_file() {
                if diff_output.trim().is_empty() && !self.is_tracked(p) {
                    if let Ok(untracked) = self.get_untracked_diff(p) {
                        if !untracked.trim().is_empty() {
                            diff_output = untracked;
                        }
                    }
                }
            } else if file_path.is_dir() {
                let prefix = if p.ends_with('/') {
                    p.to_string()
                } else {
                    format!("{}/", p)
                };
                if let Ok(uncommitted_statuses) = self.get_status() {
                    for status in uncommitted_statuses {
                        if status.status_code().contains('?') && status.path.starts_with(&prefix) {
                            if let Ok(untracked) = self.get_untracked_diff(&status.path) {
                                if !untracked.trim().is_empty() {
                                    if !diff_output.is_empty() && !diff_output.ends_with('\n') {
                                        diff_output.push('\n');
                                    }
                                    diff_output.push_str(&untracked);
                                }
                            }
                        }
                    }
                }
            }
        } else if let Ok(uncommitted_statuses) = self.get_status() {
            for status in uncommitted_statuses {
                if status.status_code().contains('?') {
                    if let Ok(untracked) = self.get_untracked_diff(&status.path) {
                        if !untracked.trim().is_empty() {
                            if !diff_output.is_empty() && !diff_output.ends_with('\n') {
                                diff_output.push('\n');
                            }
                            diff_output.push_str(&untracked);
                        }
                    }
                }
            }
        }

        Ok(diff_output)
    }

    /// Retrieve patch diff for an untracked file (`git diff --no-index /dev/null <path>`).
    pub fn get_untracked_diff(&self, path: &str) -> Result<String, GitError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.work_dir)
            .args(["diff", "--no-index", "/dev/null", path])
            .output()?;

        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(diff)
    }

    /// Check if a path is tracked in Git (`git ls-files --error-unmatch <path>`).
    pub fn is_tracked(&self, path: &str) -> bool {
        self.run_git(&["ls-files", "--error-unmatch", path]).is_ok()
    }

    /// Retrieve raw content of a file at a specific commit (`git show <commit_hash>:<path>`).
    pub fn get_file_at_commit(&self, commit_hash: &str, path: &str) -> Result<String, GitError> {
        let target = format!("{}:{}", commit_hash, path);
        self.run_git(&["show", &target])
    }

    /// Like `get_diff_between`, for a file whose path differs between the two states. The
    /// two versions are compared as blobs, so the result does not depend on git recognising
    /// the rename, which it often cannot across distant history.
    pub fn get_diff_between_paths(
        &self,
        from_hash: Option<&str>,
        to_hash: Option<&str>,
        from_path: &str,
        to_path: &str,
    ) -> Result<String, GitError> {
        if from_path == to_path {
            return self.get_diff_between(from_hash, to_hash, to_path);
        }
        match (from_hash, to_hash) {
            (Some(h1), Some(h2)) => {
                let from_blob = format!("{}:{}", h1, from_path);
                let to_blob = format!("{}:{}", h2, to_path);
                self.run_git(&["diff", &from_blob, &to_blob])
            }
            (Some(h1), None) => {
                let from_blob = format!("{}:{}", h1, from_path);
                self.run_git(&["diff", &from_blob, to_path])
            }
            (None, Some(h2)) => {
                // git can only put the working-tree file on the new side of a blob diff, so
                // diff the other way round and flip the result.
                let to_blob = format!("{}:{}", h2, to_path);
                let forward = self.run_git(&["diff", &to_blob, from_path])?;
                Ok(super::diff_parser::reverse_unified_diff(&forward))
            }
            (None, None) => Ok(String::new()),
        }
    }

    /// Retrieve patch diff between two commits/states for a specific path (`git diff <from> <to> -- <path>`).
    pub fn get_diff_between(
        &self,
        from_hash: Option<&str>,
        to_hash: Option<&str>,
        path: &str,
    ) -> Result<String, GitError> {
        match (from_hash, to_hash) {
            (Some(h1), Some(h2)) => {
                if h1 == h2 {
                    Ok(String::new())
                } else {
                    self.run_git(&["diff", h1, h2, "--", path])
                }
            }
            (Some(h1), None) => self.run_git(&["diff", h1, "--", path]),
            (None, Some(h2)) => self.run_git(&["diff", "-R", h2, "--", path]),
            (None, None) => Ok(String::new()),
        }
    }
}
