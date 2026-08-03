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

#[derive(Debug, Clone, Copy)]
pub struct TimelineJumpRequest<'a> {
    pub repo_path: &'a Path,
    pub repo: Option<&'a GitRepo>,
    pub file_path: &'a str,
    pub source_lines: &'a [String],
    pub cursor_line: usize,
    pub current_commit_hash: Option<&'a str>,
    pub head_commit_hash: Option<&'a str>,
    pub scope: JumpScope,
    pub direction: JumpDirection,
    pub cached_commits: Option<&'a [crate::git::CommitInfo]>,
    pub is_dirty: bool,
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
    pub fn jump(&self, req: TimelineJumpRequest<'_>) -> Result<Option<TimelineJumpResult>, String> {
        let repo_owned;
        let repo = match req.repo {
            Some(r) => r,
            None => {
                repo_owned = GitRepo::open(req.repo_path).map_err(|e| e.to_string())?;
                &repo_owned
            }
        };

        let source_code = req.source_lines.join("\n");
        let mut function_range: Option<(usize, usize)> = None;

        // 1. Fetch relevant commit history for the requested scope
        let commits = if let Some(cached) = req.cached_commits {
            if !cached.is_empty() {
                cached.to_vec()
            } else {
                match req.scope {
                    JumpScope::Commit => repo
                        .get_commit_history(None)
                        .map_err(|e| format!("Failed to get commit history: {:?}", e))?,
                    JumpScope::File => repo.get_file_commits(req.file_path, None).map_err(|e| {
                        format!("Failed to get file commits for {}: {:?}", req.file_path, e)
                    })?,
                    JumpScope::Function => {
                        let range = find_enclosing_function_range(
                            &self.grammar_registry,
                            req.file_path,
                            &source_code,
                            req.cursor_line,
                        )
                        .map_err(|e| format!("Tree-sitter error: {}", e))?;

                        match range {
                            Some((start_l, end_l)) => {
                                function_range = Some((start_l, end_l));
                                repo.get_line_commits(req.file_path, start_l, end_l, None)
                                    .map_err(|e| {
                                        format!(
                                            "Failed to get function commits ({}-{}) for {}: {:?}",
                                            start_l, end_l, req.file_path, e
                                        )
                                    })?
                            }
                            None => {
                                return Err(format!(
                                    "No enclosing function found at line {} in {}",
                                    req.cursor_line, req.file_path
                                ));
                            }
                        }
                    }
                    JumpScope::Line => repo
                        .get_line_commits(req.file_path, req.cursor_line, req.cursor_line, None)
                        .map_err(|e| {
                            format!(
                                "Failed to get line commits for line {} in {}: {:?}",
                                req.cursor_line, req.file_path, e
                            )
                        })?,
                }
            }
        } else {
            match req.scope {
                JumpScope::Commit => repo
                    .get_commit_history(None)
                    .map_err(|e| format!("Failed to get commit history: {:?}", e))?,
                JumpScope::File => repo.get_file_commits(req.file_path, None).map_err(|e| {
                    format!("Failed to get file commits for {}: {:?}", req.file_path, e)
                })?,
                JumpScope::Function => {
                    let range = find_enclosing_function_range(
                        &self.grammar_registry,
                        req.file_path,
                        &source_code,
                        req.cursor_line,
                    )
                    .map_err(|e| format!("Tree-sitter error: {}", e))?;

                    match range {
                        Some((start_l, end_l)) => {
                            function_range = Some((start_l, end_l));
                            repo.get_line_commits(req.file_path, start_l, end_l, None)
                                .map_err(|e| {
                                    format!(
                                        "Failed to get function commits ({}-{}) for {}: {:?}",
                                        start_l, end_l, req.file_path, e
                                    )
                                })?
                        }
                        None => {
                            return Err(format!(
                                "No enclosing function found at line {} in {}",
                                req.cursor_line, req.file_path
                            ));
                        }
                    }
                }
                JumpScope::Line => repo
                    .get_line_commits(req.file_path, req.cursor_line, req.cursor_line, None)
                    .map_err(|e| {
                        format!(
                            "Failed to get line commits for line {} in {}: {:?}",
                            req.cursor_line, req.file_path, e
                        )
                    })?,
            }
        };

        if commits.is_empty() {
            return Err(format!(
                "No commit history found for {} in {:?} scope",
                req.file_path, req.scope
            ));
        }

        // 2. Locate position in commit list based on current_commit_hash
        let current_idx = req.current_commit_hash.and_then(|hash| {
            commits.iter().position(|c| {
                c.hash == hash || c.short_hash == hash || hash.starts_with(&c.short_hash)
            })
        });

        // 3. Determine target commit index or return to working copy
        let target_commit_info = match (req.current_commit_hash, current_idx) {
            (None, _) => match req.direction {
                JumpDirection::Previous => {
                    if req.is_dirty {
                        Some(&commits[0])
                    } else {
                        let actual_head = match req.head_commit_hash {
                            Some(h) => Some(h.to_string()),
                            None => repo
                                .get_commit_history(Some(1))
                                .ok()
                                .and_then(|history| history.into_iter().next().map(|c| c.hash)),
                        };
                        let first_is_head = actual_head.map_or(false, |head| {
                            commits[0].hash == head
                                || commits[0].short_hash == head
                                || head.starts_with(&commits[0].short_hash)
                                || commits[0].short_hash.starts_with(&head)
                        });
                        if first_is_head && commits.len() > 1 {
                            Some(&commits[1])
                        } else {
                            Some(&commits[0])
                        }
                    }
                }
                JumpDirection::Next => {
                    return Err(format!(
                        "Already at latest working state for {}",
                        req.file_path
                    ));
                }
            },
            (Some(_), Some(idx)) => match req.direction {
                JumpDirection::Previous => {
                    if idx + 1 < commits.len() {
                        Some(&commits[idx + 1])
                    } else {
                        return Err(format!(
                            "Already at oldest commit in {:?} timeline for {}",
                            req.scope, req.file_path
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
            (Some(hash), None) => {
                let full_history = repo.get_commit_history(None).unwrap_or_default();
                let cur_pos = full_history.iter().position(|c| {
                    c.hash == hash || c.short_hash == hash || hash.starts_with(&c.short_hash)
                });

                match cur_pos {
                    Some(pos) => match req.direction {
                        JumpDirection::Previous => {
                            let candidate = commits.iter().find(|cand| {
                                if let Some(cand_pos) = full_history.iter().position(|c| {
                                    c.hash == cand.hash
                                        || c.short_hash == cand.short_hash
                                        || cand.hash.starts_with(&c.short_hash)
                                }) {
                                    cand_pos > pos
                                } else {
                                    false
                                }
                            });
                            if let Some(cand) = candidate {
                                Some(cand)
                            } else {
                                return Err(format!(
                                    "Already at oldest commit in {:?} timeline for {}",
                                    req.scope, req.file_path
                                ));
                            }
                        }
                        JumpDirection::Next => {
                            commits.iter().rev().find(|cand| {
                                if let Some(cand_pos) = full_history.iter().position(|c| {
                                    c.hash == cand.hash
                                        || c.short_hash == cand.short_hash
                                        || cand.hash.starts_with(&c.short_hash)
                                }) {
                                    cand_pos < pos
                                } else {
                                    false
                                }
                            })
                        }
                    },
                    None => match req.direction {
                        JumpDirection::Previous => commits.last(),
                        JumpDirection::Next => None,
                    },
                }
            }
        };

        // If stepping NEXT past newest commit -> return None (signals reset to working copy)
        let target_commit = match target_commit_info {
            Some(c) => c,
            None => return Ok(None),
        };

        // 4. Load file contents at target commit (fallback to first modified file if file didn't exist in Commit scope)
        let mut target_file_path = req.file_path.to_string();
        let content = match repo.get_file_at_commit(&target_commit.hash, req.file_path) {
            Ok(content) => content,
            Err(e) => {
                if req.scope == JumpScope::Commit {
                    let commit_files = repo.get_commit_files(&target_commit.hash).map_err(|cf_err| {
                        format!("Failed to read file {} at commit {}: {:?} (failed fetching commit files: {:?})", req.file_path, target_commit.short_hash, e, cf_err)
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
                            req.file_path, target_commit.short_hash
                        ));
                    }
                } else {
                    return Err(format!(
                        "Failed to read file {} at commit {}: {:?}",
                        req.file_path, target_commit.short_hash, e
                    ));
                }
            }
        };

        let code_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let scope_desc = match req.scope {
            JumpScope::Commit => "COMMIT".to_string(),
            JumpScope::File => "FILE".to_string(),
            JumpScope::Function => {
                if let Some((s, e)) = function_range {
                    format!("FUNCTION (lines {}-{})", s, e)
                } else {
                    "FUNCTION".to_string()
                }
            }
            JumpScope::Line => format!("LINE {}", req.cursor_line),
        };

        let dir_desc = match req.direction {
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
            scope: req.scope,
            line_range: function_range,
        }))
    }
}
