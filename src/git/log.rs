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

        // `git log -L` follows the range across renames, so a commit may know the file under
        // an older path. The only place git reports that path is the patch header, so the
        // patch is kept and each record is prefixed with a marker to separate it from the
        // previous commit's patch.
        let line_range_arg = format!("-L{},{}:{}", start, end, clean_path);
        let mut args = vec![
            "log",
            "--format=%x1d%H%x1f%h%x1f%an%x1f%ae%x1f%aI%x1f%s%x1f%b%x1e",
            &line_range_arg,
        ];
        let max_count_str;
        if let Some(count) = max_count {
            max_count_str = format!("-n{}", count);
            args.push(&max_count_str);
        }

        let output = self.run_git(&args)?;
        parse_line_commit_log(&output)
    }
}

/// Parse `git log -L` output where each commit is `\x1d<fields>\x1e<patch>`, recording the
/// path the file had at each commit.
pub fn parse_line_commit_log(raw_log: &str) -> Result<Vec<CommitInfo>, GitError> {
    let mut commits = Vec::new();

    for chunk in raw_log.split('\x1d') {
        let (record, patch) = match chunk.split_once('\x1e') {
            Some(parts) => parts,
            None => continue,
        };
        if let Some(mut commit) = parse_commit_log(record)?.into_iter().next() {
            commit.path = path_from_patch_header(patch);
            commits.push(commit);
        }
    }

    Ok(commits)
}

/// Extract the post-image path from the `+++ b/<path>` line of a patch header.
fn path_from_patch_header(patch: &str) -> Option<String> {
    for line in patch.lines() {
        if line.starts_with("@@") {
            break;
        }
        if let Some(path) = line.strip_prefix("+++ b/") {
            let path = path.trim_end_matches('\t');
            return (!path.is_empty()).then(|| path.to_string());
        }
    }
    None
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
            path: None,
        });
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_commit_log_records_path_per_commit() {
        let raw = concat!(
            "\x1daaa111\x1faaa\x1fAnn\x1fann@example.com\x1f2026-01-02\x1fNewer\x1f\x1e\n",
            "\n",
            "diff --git a/src/new name.rs b/src/new name.rs\n",
            "--- a/src/new name.rs\n",
            "+++ b/src/new name.rs\t\n",
            "@@ -1,1 +1,1 @@\n",
            "-old\n",
            "+++ b/not-a-header\n",
            "\x1dbbb222\x1fbbb\x1fBob\x1fbob@example.com\x1f2026-01-01\x1fOlder\x1fbody text\x1e\n",
            "\n",
            "diff --git a/old.rs b/old.rs\n",
            "--- /dev/null\n",
            "+++ b/old.rs\n",
            "@@ -0,0 +1,1 @@\n",
            "+old\n",
        );

        let commits = parse_line_commit_log(raw).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Newer");
        assert_eq!(commits[0].path.as_deref(), Some("src/new name.rs"));
        assert_eq!(commits[1].summary, "Older");
        assert_eq!(commits[1].body, "body text");
        assert_eq!(commits[1].path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn test_parse_commit_log_leaves_path_unset() {
        let raw = "aaa111\x1faaa\x1fAnn\x1fann@example.com\x1f2026-01-02\x1fSubject\x1f\x1e";
        let commits = parse_commit_log(raw).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].path, None);
    }
}
