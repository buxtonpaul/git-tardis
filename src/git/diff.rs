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

    /// Retrieve working tree diff relative to HEAD (`git diff HEAD`).
    pub fn get_working_diff(&self, path: Option<&str>) -> Result<String, GitError> {
        let mut args = vec!["diff", "HEAD"];
        if let Some(p) = path {
            args.push("--");
            args.push(p);
        }

        match self.run_git(&args) {
            Ok(output) => Ok(output),
            Err(_) => {
                // Fallback for initial commit state where HEAD might not resolve yet
                let mut fallback_args = vec!["diff"];
                if let Some(p) = path {
                    fallback_args.push("--");
                    fallback_args.push(p);
                }
                self.run_git(&fallback_args)
            }
        }
    }

    /// Retrieve raw content of a file at a specific commit (`git show <commit_hash>:<path>`).
    pub fn get_file_at_commit(&self, commit_hash: &str, path: &str) -> Result<String, GitError> {
        let target = format!("{}:{}", commit_hash, path);
        self.run_git(&["show", &target])
    }
}
