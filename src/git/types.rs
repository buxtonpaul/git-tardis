use std::fmt;

#[derive(Debug)]
pub enum GitError {
    NotARepository(String),
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    Io(std::io::Error),
    ParseError(String),
    NotFound(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotARepository(path) => write!(f, "Not a git repository: {}", path),
            GitError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => {
                write!(
                    f,
                    "Git command '{}' failed (code {:?}): {}",
                    command,
                    exit_code,
                    stderr.trim()
                )
            }
            GitError::Io(err) => write!(f, "IO error: {}", err),
            GitError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GitError::NotFound(item) => write!(f, "Not found: {}", item),
        }
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(err: std::io::Error) -> Self {
        GitError::Io(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub summary: String,
    pub body: String,
}

impl CommitInfo {
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
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub index_status: char,
    pub worktree_status: char,
}

impl FileStatus {
    pub fn new(path: impl Into<String>, index_status: char, worktree_status: char) -> Self {
        Self {
            path: path.into(),
            index_status,
            worktree_status,
        }
    }

    pub fn is_staged(&self) -> bool {
        self.index_status != ' ' && self.index_status != '?' && self.index_status != '!'
    }

    pub fn is_modified(&self) -> bool {
        self.worktree_status == 'M' || self.index_status == 'M'
    }

    pub fn is_untracked(&self) -> bool {
        self.index_status == '?' && self.worktree_status == '?'
    }

    pub fn status_code(&self) -> String {
        format!("{}{}", self.index_status, self.worktree_status)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlameLine {
    pub commit_hash: String,
    pub orig_line: usize,
    pub final_line: usize,
    pub author: String,
    pub author_mail: String,
    #[serde(default)]
    pub author_time: u64,
    pub summary: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlameHunk {
    pub commit_hash: String,
    pub start_line: usize,
    pub line_count: usize,
    pub author: String,
    pub author_mail: String,
    pub summary: String,
}
