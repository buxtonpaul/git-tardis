use std::collections::HashMap;

use super::{BlameHunk, BlameLine, GitError, GitRepo};

#[derive(Default, Debug, Clone)]
struct CommitMeta {
    author: String,
    author_mail: String,
    summary: String,
}

impl GitRepo {
    /// Retrieve line-by-line blame information for a file (`git blame -p`).
    pub fn get_blame(
        &self,
        path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<Vec<BlameLine>, GitError> {
        let mut args = vec!["blame", "-p"];
        let line_range_arg;

        if let Some(start) = start_line {
            let end = end_line.unwrap_or(start);
            line_range_arg = format!("-L{},{}", start, end);
            args.push(&line_range_arg);
        }

        args.push("--");
        args.push(path);

        let output = self.run_git(&args)?;
        parse_blame_porcelain(&output)
    }

    /// Retrieve aggregated blame hunks for a file.
    pub fn get_blame_hunks(
        &self,
        path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<Vec<BlameHunk>, GitError> {
        let blame_lines = self.get_blame(path, start_line, end_line)?;
        Ok(aggregate_blame_hunks(&blame_lines))
    }
}

pub fn parse_blame_porcelain(raw: &str) -> Result<Vec<BlameLine>, GitError> {
    let mut commit_headers: HashMap<String, CommitMeta> = HashMap::new();
    let mut blame_lines = Vec::new();

    let mut current_hash = String::new();
    let mut current_orig_line = 0usize;
    let mut current_final_line = 0usize;

    for line in raw.lines() {
        if let Some(content_str) = line.strip_prefix('\t') {
            // Line content line - completes current blame entry
            let content = content_str.to_string();
            let meta = commit_headers
                .get(&current_hash)
                .cloned()
                .unwrap_or_default();

            blame_lines.push(BlameLine {
                commit_hash: current_hash.clone(),
                orig_line: current_orig_line,
                final_line: current_final_line,
                author: meta.author,
                author_mail: meta.author_mail,
                summary: meta.summary,
                content,
            });
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // Check if line is header: <commit-hash> <orig-line> <final-line> [num-lines]
        if parts.len() >= 3 && is_hex_hash(parts[0]) {
            current_hash = parts[0].to_string();
            current_orig_line = parts[1].parse().unwrap_or(0);
            current_final_line = parts[2].parse().unwrap_or(0);
            commit_headers.entry(current_hash.clone()).or_default();
            continue;
        }

        // Otherwise key-value metadata line
        if let Some((key, value)) = line.split_once(' ') {
            if let Some(meta) = commit_headers.get_mut(&current_hash) {
                match key {
                    "author" => meta.author = value.trim().to_string(),
                    "author-mail" => {
                        let mail = value.trim();
                        meta.author_mail = mail
                            .strip_prefix('<')
                            .and_then(|m| m.strip_suffix('>'))
                            .unwrap_or(mail)
                            .to_string();
                    }
                    "summary" => meta.summary = value.trim().to_string(),
                    _ => {}
                }
            }
        }
    }

    Ok(blame_lines)
}

pub fn aggregate_blame_hunks(lines: &[BlameLine]) -> Vec<BlameHunk> {
    let mut hunks = Vec::new();
    if lines.is_empty() {
        return hunks;
    }

    let mut current_hunk = BlameHunk {
        commit_hash: lines[0].commit_hash.clone(),
        start_line: lines[0].final_line,
        line_count: 1,
        author: lines[0].author.clone(),
        author_mail: lines[0].author_mail.clone(),
        summary: lines[0].summary.clone(),
    };

    for line in &lines[1..] {
        if line.commit_hash == current_hunk.commit_hash {
            current_hunk.line_count += 1;
        } else {
            hunks.push(current_hunk);
            current_hunk = BlameHunk {
                commit_hash: line.commit_hash.clone(),
                start_line: line.final_line,
                line_count: 1,
                author: line.author.clone(),
                author_mail: line.author_mail.clone(),
                summary: line.summary.clone(),
            };
        }
    }
    hunks.push(current_hunk);

    hunks
}

fn is_hex_hash(s: &str) -> bool {
    s.len() >= 7 && s.chars().all(|c| c.is_ascii_hexdigit())
}
