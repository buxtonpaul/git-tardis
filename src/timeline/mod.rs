use std::path::Path;

use crate::git::GitRepo;
use crate::treesitter::registry::GrammarRegistry;
use crate::treesitter::scope::find_enclosing_function_range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpScope {
    Commit,
    File,
    Function,
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpDirection {
    Next,
    Previous,
}

#[derive(Debug, Clone)]
pub struct TimelineJumpResult {
    pub commit_hash: String,
    pub short_hash: String,
    pub commit_summary: String,
    pub file_path: String,
    pub code_lines: Vec<String>,
    pub status_message: String,
    pub scope: JumpScope,
    pub line_range: Option<(usize, usize)>,
}

pub struct TimelineNavigator {
    grammar_registry: GrammarRegistry,
}

impl Default for TimelineNavigator {
    fn default() -> Self {
        Self::new()
    }
}

impl TimelineNavigator {
    pub fn new() -> Self {
        Self {
            grammar_registry: GrammarRegistry::new(),
        }
    }

    /// Perform a time travel jump for the given repo, file, cursor position, and jump mode.
    pub fn jump(
        &self,
        repo_path: &Path,
        file_path: &str,
        source_lines: &[String],
        cursor_line: usize,
        current_commit_hash: Option<&str>,
        scope: JumpScope,
        direction: JumpDirection,
    ) -> Result<Option<TimelineJumpResult>, String> {
        let repo = GitRepo::open(repo_path).map_err(|e| e.to_string())?;

        let source_code = source_lines.join("\n");
        let mut function_range: Option<(usize, usize)> = None;

        // 1. Fetch relevant commit history for the requested scope
        let commits = match scope {
            JumpScope::Commit => repo
                .get_commit_history(None)
                .map_err(|e| format!("Failed to get commit history: {:?}", e))?,
            JumpScope::File => repo
                .get_file_commits(file_path, None)
                .map_err(|e| format!("Failed to get file commits for {}: {:?}", file_path, e))?,
            JumpScope::Function => {
                let range = find_enclosing_function_range(
                    &self.grammar_registry,
                    file_path,
                    &source_code,
                    cursor_line,
                )
                .map_err(|e| format!("Tree-sitter error: {}", e))?;

                match range {
                    Some((start_l, end_l)) => {
                        function_range = Some((start_l, end_l));
                        repo.get_line_commits(file_path, start_l, end_l, None)
                            .map_err(|e| {
                                format!(
                                    "Failed to get function commits ({}-{}) for {}: {:?}",
                                    start_l, end_l, file_path, e
                                )
                            })?
                    }
                    None => {
                        return Err(format!(
                            "No enclosing function found at line {} in {}",
                            cursor_line, file_path
                        ));
                    }
                }
            }
            JumpScope::Line => repo
                .get_line_commits(file_path, cursor_line, cursor_line, None)
                .map_err(|e| {
                    format!(
                        "Failed to get line commits for line {} in {}: {:?}",
                        cursor_line, file_path, e
                    )
                })?,
        };

        if commits.is_empty() {
            return Err(format!(
                "No commit history found for {} in {:?} scope",
                file_path, scope
            ));
        }

        // 2. Locate position in commit list based on current_commit_hash
        let current_idx = current_commit_hash.and_then(|hash| {
            commits.iter().position(|c| {
                c.hash == hash || c.short_hash == hash || hash.starts_with(&c.short_hash)
            })
        });

        // 3. Determine target commit index or return to working copy
        let target_commit_info = match (current_commit_hash, current_idx) {
            (None, _) => match direction {
                JumpDirection::Previous => Some(&commits[0]),
                JumpDirection::Next => {
                    return Err(format!("Already at latest working state for {}", file_path));
                }
            },
            (Some(_), Some(idx)) => match direction {
                JumpDirection::Previous => {
                    if idx + 1 < commits.len() {
                        Some(&commits[idx + 1])
                    } else {
                        return Err(format!(
                            "Already at oldest commit in {:?} timeline for {}",
                            scope, file_path
                        ));
                    }
                }
                JumpDirection::Next => {
                    if idx > 0 {
                        Some(&commits[idx - 1])
                    } else {
                        // Stepping NEXT past newest commit -> return None to signal return to working directory
                        None
                    }
                }
            },
            (Some(_), None) => match direction {
                JumpDirection::Previous => Some(&commits[0]),
                JumpDirection::Next => None,
            },
        };

        // If stepping NEXT past newest commit -> return None (signals reset to working copy)
        let target_commit = match target_commit_info {
            Some(c) => c,
            None => return Ok(None),
        };

        // 4. Load file contents at target commit (fallback to first modified file if file didn't exist in Commit scope)
        let mut target_file_path = file_path.to_string();
        let content = match repo.get_file_at_commit(&target_commit.hash, file_path) {
            Ok(content) => content,
            Err(e) => {
                if scope == JumpScope::Commit {
                    let commit_files = repo.get_commit_files(&target_commit.hash).map_err(|cf_err| {
                        format!("Failed to read file {} at commit {}: {:?} (failed fetching commit files: {:?})", file_path, target_commit.short_hash, e, cf_err)
                    })?;
                    if let Some(first) =
                        commit_files.iter().find(|f| !f.status_code().contains('D'))
                    {
                        target_file_path = first.path.clone();
                        repo.get_file_at_commit(&target_commit.hash, &target_file_path)
                            .map_err(|read_err| {
                                format!(
                                    "Failed to read fallback file {} at commit {}: {:?}",
                                    target_file_path, target_commit.short_hash, read_err
                                )
                            })?
                    } else {
                        return Err(format!(
                            "File {} did not exist at commit {}, and no valid modified file found",
                            file_path, target_commit.short_hash
                        ));
                    }
                } else {
                    return Err(format!(
                        "Failed to read file {} at commit {}: {:?}",
                        file_path, target_commit.short_hash, e
                    ));
                }
            }
        };

        let code_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let scope_desc = match scope {
            JumpScope::Commit => "COMMIT".to_string(),
            JumpScope::File => "FILE".to_string(),
            JumpScope::Function => {
                if let Some((s, e)) = function_range {
                    format!("FUNCTION (lines {}-{})", s, e)
                } else {
                    "FUNCTION".to_string()
                }
            }
            JumpScope::Line => format!("LINE {}", cursor_line),
        };

        let dir_desc = match direction {
            JumpDirection::Previous => "PREVIOUS",
            JumpDirection::Next => "NEXT",
        };

        let status_message = format!(
            "Jumped [{}] ({}) -> {} ({})",
            dir_desc, scope_desc, target_commit.short_hash, target_commit.summary
        );

        Ok(Some(TimelineJumpResult {
            commit_hash: target_commit.hash.clone(),
            short_hash: target_commit.short_hash.clone(),
            commit_summary: target_commit.summary.clone(),
            file_path: target_file_path,
            code_lines,
            status_message,
            scope,
            line_range: function_range,
        }))
    }
}
