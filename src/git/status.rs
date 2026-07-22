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

    /// Query repository status (`git status --porcelain=v1 -z`).
    pub fn get_status(&self) -> Result<Vec<FileStatus>, GitError> {
        let bytes = self.run_git_bytes(&["status", "--porcelain=v1", "-z"])?;

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
}
