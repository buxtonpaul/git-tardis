use serde::{Deserialize, Serialize};
use std::fmt;

/// Active in-file navigation input prompt modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPrompt {
    GotoLine,
    SearchText,
    SearchSymbol,
}

/// Mode for displaying file contents vs git diff in the File Viewer panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileViewMode {
    #[default]
    Full,
    Diff,
}

impl FileViewMode {
    pub fn name(&self) -> &'static str {
        match self {
            FileViewMode::Full => "FULL",
            FileViewMode::Diff => "DIFF",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            FileViewMode::Full => FileViewMode::Diff,
            FileViewMode::Diff => FileViewMode::Full,
        }
    }
}

/// Filter mode for the Timeline sidebar view (All commits vs Candidate commits)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineFilter {
    #[default]
    All,
    Candidates,
}

impl TimelineFilter {
    pub fn name(&self) -> &'static str {
        match self {
            TimelineFilter::All => "ALL",
            TimelineFilter::Candidates => "CANDIDATES",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            TimelineFilter::All => TimelineFilter::Candidates,
            TimelineFilter::Candidates => TimelineFilter::All,
        }
    }
}

/// Domain representation of a commit summary in navigation lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitSummary {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
}

impl CommitSummary {
    pub fn new(
        hash: impl Into<String>,
        short_hash: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            hash: hash.into(),
            short_hash: short_hash.into(),
            message: message.into(),
        }
    }

    pub fn matches_hash(&self, target_hash: &str) -> bool {
        if target_hash.is_empty() {
            return false;
        }
        self.hash == target_hash
            || self.short_hash == target_hash
            || self.hash.starts_with(target_hash)
            || self.short_hash.starts_with(target_hash)
            || target_hash.starts_with(&self.short_hash)
            || target_hash.starts_with(&self.hash)
    }

    pub fn matches_candidate(&self, candidate: &CommitSummary) -> bool {
        self.matches_hash(&candidate.hash) || self.matches_hash(&candidate.short_hash)
    }
}

impl fmt::Display for CommitSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let display_hash = if !self.short_hash.is_empty() {
            &self.short_hash
        } else if self.hash.len() >= 7 {
            &self.hash[..7]
        } else {
            &self.hash
        };
        write!(f, "{} {}", display_hash, self.message)
    }
}

impl From<crate::git::CommitInfo> for CommitSummary {
    fn from(c: crate::git::CommitInfo) -> Self {
        Self {
            hash: c.hash,
            short_hash: c.short_hash,
            message: c.summary,
        }
    }
}

impl From<&crate::git::CommitInfo> for CommitSummary {
    fn from(c: &crate::git::CommitInfo) -> Self {
        Self {
            hash: c.hash.clone(),
            short_hash: c.short_hash.clone(),
            message: c.summary.clone(),
        }
    }
}

impl From<(String, String)> for CommitSummary {
    fn from((hash, message): (String, String)) -> Self {
        let short_hash = if hash.len() >= 7 {
            hash[..7].to_string()
        } else {
            hash.clone()
        };
        Self {
            hash,
            short_hash,
            message,
        }
    }
}

impl From<(&str, &str)> for CommitSummary {
    fn from((hash, message): (&str, &str)) -> Self {
        let hash_str = hash.to_string();
        let short_hash = if hash_str.len() >= 7 {
            hash_str[..7].to_string()
        } else {
            hash_str.clone()
        };
        Self {
            hash: hash_str,
            short_hash,
            message: message.to_string(),
        }
    }
}

/// Domain representation of a modified file entry in file list panels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiedFileEntry {
    pub path: String,
    pub status: String,
}

impl ModifiedFileEntry {
    pub fn new(path: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            status: status.into(),
        }
    }

    pub fn display_string(&self) -> String {
        let trimmed_status = self.status.trim();
        if trimmed_status.is_empty() {
            self.path.clone()
        } else {
            format!("{} ({})", self.path, trimmed_status)
        }
    }
}

impl fmt::Display for ModifiedFileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_string())
    }
}

impl From<crate::git::FileStatus> for ModifiedFileEntry {
    fn from(s: crate::git::FileStatus) -> Self {
        let status = s.status_code().trim().to_string();
        Self {
            path: s.path,
            status,
        }
    }
}

impl From<&crate::git::FileStatus> for ModifiedFileEntry {
    fn from(s: &crate::git::FileStatus) -> Self {
        Self {
            path: s.path.clone(),
            status: s.status_code().trim().to_string(),
        }
    }
}

impl From<String> for ModifiedFileEntry {
    fn from(s: String) -> Self {
        if let Some(idx) = s.rfind('(') {
            if s.ends_with(')') {
                let path = s[..idx].trim().to_string();
                let status = s[idx + 1..s.len() - 1].trim().to_string();
                return Self { path, status };
            }
        }
        Self {
            path: s.trim().to_string(),
            status: String::new(),
        }
    }
}

impl From<&str> for ModifiedFileEntry {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommitInfo, FileStatus};

    #[test]
    fn test_commit_summary_from_commit_info() {
        let info = CommitInfo {
            hash: "1234567890abcdef1234567890abcdef12345678".to_string(),
            short_hash: "1234567".to_string(),
            author: "Author".to_string(),
            email: "author@example.com".to_string(),
            date: "2026-01-01".to_string(),
            summary: "Initial commit".to_string(),
            body: "".to_string(),
        };

        let summary = CommitSummary::from(info);
        assert_eq!(summary.hash, "1234567890abcdef1234567890abcdef12345678");
        assert_eq!(summary.short_hash, "1234567");
        assert_eq!(summary.message, "Initial commit");
        assert_eq!(summary.to_string(), "1234567 Initial commit");
    }

    #[test]
    fn test_commit_summary_matching() {
        let summary = CommitSummary::new("1234567890abcdef", "1234567", "Fix issue");

        assert!(summary.matches_hash("1234567890abcdef"));
        assert!(summary.matches_hash("1234567"));
        assert!(summary.matches_hash("12345678"));
        assert!(!summary.matches_hash("9999999"));

        let candidate = CommitSummary::new("1234567", "1234567", "Fix issue");
        assert!(summary.matches_candidate(&candidate));
    }

    #[test]
    fn test_modified_file_entry_from_file_status() {
        let status = FileStatus::new("src/main.rs", ' ', 'M');
        let entry = ModifiedFileEntry::from(status);

        assert_eq!(entry.path, "src/main.rs");
        assert_eq!(entry.status, "M");
        assert_eq!(entry.display_string(), "src/main.rs (M)");
        assert_eq!(entry.to_string(), "src/main.rs (M)");
    }

    #[test]
    fn test_modified_file_entry_from_formatted_string() {
        let entry = ModifiedFileEntry::from("src/app/mod.rs (M)");
        assert_eq!(entry.path, "src/app/mod.rs");
        assert_eq!(entry.status, "M");
        assert_eq!(entry.display_string(), "src/app/mod.rs (M)");

        let plain_entry = ModifiedFileEntry::from("README.md");
        assert_eq!(plain_entry.path, "README.md");
        assert_eq!(plain_entry.status, "");
        assert_eq!(plain_entry.display_string(), "README.md");
    }
}
