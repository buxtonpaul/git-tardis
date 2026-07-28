use super::{CommitInfo, GitError, GitRepo};

impl GitRepo {
    /// Retrieve full commit history for the current branch/repository.
    pub fn get_commit_history(
        &self,
        max_count: Option<usize>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let mut args = vec![
            "log",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%b%x1e",
        ];
        let max_count_str;
        if let Some(count) = max_count {
            max_count_str = format!("-n{}", count);
            args.push(&max_count_str);
        }

        let output = self.run_git(&args)?;
        parse_commit_log(&output)
    }

    /// Retrieve commit history that modified a specific file path.
    pub fn get_file_commits(
        &self,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let clean_path = path.trim().trim_start_matches("./");
        if clean_path.is_empty() {
            return Ok(Vec::new());
        }

        let mut args = vec![
            "log",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%b%x1e",
        ];
        let max_count_str;
        if let Some(count) = max_count {
            max_count_str = format!("-n{}", count);
            args.push(&max_count_str);
        }
        args.push("--");
        args.push(clean_path);

        let output = self.run_git(&args)?;
        parse_commit_log(&output)
    }

    /// Retrieve commit history that modified a specific line range in a file.
    pub fn get_line_commits(
        &self,
        path: &str,
        start_line: usize,
        end_line: usize,
        max_count: Option<usize>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let clean_path = path.trim().trim_start_matches("./");
        if clean_path.is_empty() {
            return Ok(Vec::new());
        }

        let start = start_line.max(1);
        let end = end_line.max(start);

        let line_range_arg = format!("-L{},{}:{}", start, end, clean_path);
        let mut args = vec![
            "log",
            "--no-patch",
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%b%x1e",
            &line_range_arg,
        ];
        let max_count_str;
        if let Some(count) = max_count {
            max_count_str = format!("-n{}", count);
            args.push(&max_count_str);
        }

        let output = self.run_git(&args)?;
        parse_commit_log(&output)
    }
}

pub fn parse_commit_log(raw_log: &str) -> Result<Vec<CommitInfo>, GitError> {
    let mut commits = Vec::new();

    for record in raw_log.split('\x1e') {
        let trimmed = record.trim();
        if trimmed.is_empty() {
            continue;
        }

        let fields: Vec<&str> = trimmed.split('\x1f').collect();
        if fields.len() < 6 {
            continue;
        }

        let hash = fields[0].trim().to_string();
        let short_hash = fields[1].trim().to_string();
        let author = fields[2].trim().to_string();
        let email = fields[3].trim().to_string();
        let date = fields[4].trim().to_string();
        let summary = fields[5].trim().to_string();
        let body = if fields.len() > 6 {
            fields[6].trim().to_string()
        } else {
            String::new()
        };

        commits.push(CommitInfo {
            hash,
            short_hash,
            author,
            email,
            date,
            summary,
            body,
        });
    }

    Ok(commits)
}
