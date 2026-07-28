use super::{FileStatus, GitError, GitRepo};

impl GitRepo {
    /// List all tracked and untracked non-ignored files in the repository (`git ls-files`).
    pub fn list_files(&self) -> Result<Vec<String>, GitError> {
        let bytes = self.run_git_bytes(&[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])?;

        let mut files = Vec::new();
        for slice in bytes.split(|&b| b == 0) {
            if slice.is_empty() {
                continue;
            }
            let path_str = String::from_utf8_lossy(slice).to_string();
            if !path_str.is_empty() {
                files.push(path_str);
            }
        }

        files.sort();
        Ok(files)
    }

    /// List all tracked files at a specific commit (`git ls-tree -r --name-only -z <commit>`).
    pub fn list_files_at_commit(&self, commit_hash: &str) -> Result<Vec<String>, GitError> {
        let bytes = self.run_git_bytes(&["ls-tree", "-r", "--name-only", "-z", commit_hash])?;

        let mut files = Vec::new();
        for slice in bytes.split(|&b| b == 0) {
            if slice.is_empty() {
                continue;
            }
            let path_str = String::from_utf8_lossy(slice).to_string();
            if !path_str.is_empty() {
                files.push(path_str);
            }
        }

        files.sort();
        Ok(files)
    }

    /// Query repository status (`git status --porcelain=v1 -z -uall`).
    pub fn get_status(&self) -> Result<Vec<FileStatus>, GitError> {
        let bytes = self.run_git_bytes(&["status", "--porcelain=v1", "-z", "-uall"])?;

        let mut statuses = Vec::new();
        let mut slices = bytes.split(|&b| b == 0).peekable();

        while let Some(slice) = slices.next() {
            if slice.is_empty() {
                continue;
            }

            let entry_str = String::from_utf8_lossy(slice).to_string();
            let chars: Vec<char> = entry_str.chars().collect();

            if chars.len() < 3 {
                continue;
            }

            let index_status = chars[0];
            let worktree_status = chars[1];
            // chars[2] is space
            let path: String = chars[3..].iter().collect();

            // Handle rename/copy where next element in NUL-separated stream is origin path
            if index_status == 'R' || index_status == 'C' {
                let _orig_path = slices.next();
            }

            statuses.push(FileStatus::new(path, index_status, worktree_status));
        }

        Ok(statuses)
    }

    /// Get list of files modified in a specific commit (`git diff-tree -z --no-commit-id --name-status -r -M --root <commit_hash>`).
    pub fn get_commit_files(&self, commit_hash: &str) -> Result<Vec<FileStatus>, GitError> {
        let bytes = self.run_git_bytes(&[
            "diff-tree",
            "-z",
            "--no-commit-id",
            "--name-status",
            "-r",
            "-M",
            "--root",
            commit_hash,
        ])?;

        let mut results = Vec::new();
        let mut slices = bytes.split(|&b| b == 0);

        while let Some(status_slice) = slices.next() {
            if status_slice.is_empty() {
                continue;
            }

            let status_str = String::from_utf8_lossy(status_slice);
            let status_char = status_str.chars().next().unwrap_or('M');

            if status_char == 'R' || status_char == 'C' {
                let _old_path = slices.next();
                if let Some(new_path_slice) = slices.next() {
                    let path = String::from_utf8_lossy(new_path_slice).to_string();
                    if !path.is_empty() {
                        results.push(FileStatus::new(path, status_char, ' '));
                    }
                }
            } else if let Some(path_slice) = slices.next() {
                let path = String::from_utf8_lossy(path_slice).to_string();
                if !path.is_empty() {
                    results.push(FileStatus::new(path, status_char, ' '));
                }
            }
        }

        Ok(results)
    }
}
