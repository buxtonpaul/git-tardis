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

    /// Like `get_commit_history`, asking git only for the hashes and subject line. The
    /// author, email, date and body of each commit are left empty. Listing a long history
    /// is dominated by the size of the output, and the timeline shows none of those fields.
    pub fn get_commit_history_brief(
        &self,
        max_count: Option<usize>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let mut args = vec!["log", "--format=%H%x1f%h%x1f%x1f%x1f%x1f%s%x1f%x1e"];
        let max_count_str;
        if let Some(count) = max_count {
            max_count_str = format!("-n{}", count);
            args.push(&max_count_str);
        }

        let output = self.run_git(&args)?;
        parse_commit_log(&output)
    }

    /// Retrieve commit history that modified a specific file path, continuing through
    /// renames of the file. Each commit records the path the file had at that commit.
    pub fn get_file_commits(
        &self,
        path: &str,
        max_count: Option<usize>,
    ) -> Result<Vec<CommitInfo>, GitError> {
        let clean_path = path.trim().trim_start_matches("./");
        if clean_path.is_empty() {
            return Ok(Vec::new());
        }

        // A directory has no single path to follow, so it keeps the plain path-limited log.
        if self.work_dir.join(clean_path).is_dir() {
            return self.file_commits_segment(None, clean_path, max_count);
        }

        // Follow the file back through renames one name at a time: list the commits under
        // the current name, and if that list ends at a commit which renamed the file,
        // carry on from that commit's parent under the previous name. Each commit records
        // the name the file had there.
        //
        // `git log --follow` does this in one command but is several times slower on a
        // large repository, and it also follows copies, so a file created with the same
        // content as another would inherit that file's history.
        let mut commits: Vec<CommitInfo> = Vec::new();
        let mut start: Option<String> = None;
        let mut name = clean_path.to_string();

        for _ in 0..MAX_FOLLOWED_RENAMES {
            let remaining = max_count.map(|limit| limit.saturating_sub(commits.len()));
            if remaining == Some(0) {
                break;
            }
            let mut segment = self.file_commits_segment(start.as_deref(), &name, remaining)?;
            let oldest = match segment.last() {
                Some(commit) => commit.hash.clone(),
                None => break,
            };
            let hit_limit = remaining == Some(segment.len());
            for commit in &mut segment {
                commit.path = Some(name.clone());
            }
            commits.append(&mut segment);

            if hit_limit {
                break;
            }
            match self.renamed_from(&oldest, &name) {
                Some(previous_name) => {
                    start = Some(format!("{}^", oldest));
                    name = previous_name;
                }
                None => break,
            }
        }

        Ok(commits)
    }

    /// Commits touching `path`, newest first, starting from `start` (HEAD when `None`).
    fn file_commits_segment(
        &self,
        start: Option<&str>,
        path: &str,
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
        if let Some(rev) = start {
            args.push(rev);
        }
        args.push("--");
        args.push(path);

        let output = self.run_git(&args)?;
        parse_commit_log(&output)
    }

    /// If `commit` created `path` by renaming another file, the name it had before.
    fn renamed_from(&self, commit: &str, path: &str) -> Option<String> {
        let bytes = self
            .run_git_bytes(&[
                "diff-tree",
                "-z",
                "--no-commit-id",
                "--name-status",
                "-r",
                "-M",
                "--root",
                commit,
            ])
            .ok()?;

        let mut fields = bytes.split(|&b| b == 0);
        while let Some(status) = fields.next() {
            if status.is_empty() {
                continue;
            }
            // Renames and copies list two paths (old, new); everything else lists one.
            if status[0] == b'R' || status[0] == b'C' {
                let old = fields.next()?;
                let new = fields.next()?;
                if status[0] == b'R' && new == path.as_bytes() {
                    return Some(String::from_utf8_lossy(old).to_string());
                }
            } else {
                fields.next()?;
            }
        }
        None
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
    parse_marked_commit_log(raw_log, path_from_patch_header)
}

/// Parse log output where each commit is `\x1d<fields>\x1e<trailer>`, using `path_of` to
/// read the file's path at that commit out of the trailer.
fn parse_marked_commit_log(
    raw_log: &str,
    path_of: fn(&str) -> Option<String>,
) -> Result<Vec<CommitInfo>, GitError> {
    let mut commits = Vec::new();

    for chunk in raw_log.split('\x1d') {
        let (record, trailer) = match chunk.split_once('\x1e') {
            Some(parts) => parts,
            None => continue,
        };
        if let Some(mut commit) = parse_commit_log(record)?.into_iter().next() {
            commit.path = path_of(trailer);
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

/// Upper bound on how many renames of one file are followed, as a guard against cycles.
const MAX_FOLLOWED_RENAMES: usize = 32;

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
