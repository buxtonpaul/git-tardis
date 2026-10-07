//! Background thread for git queries that follow the cursor, so key handling never waits on
//! a subprocess.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use super::{CandidateQueryKey, CommitSummary};
use crate::git::{BlameLine, GitError, GitRepo};

/// Commit limit applied to candidate history queries.
pub const CANDIDATE_COMMIT_LIMIT: usize = 200;

/// How many commits the timeline loads before falling back to the whole history.
pub const EXTENDED_HISTORY_LIMIT: usize = 1000;

/// Key identifying a full-file blame: file path and the commit it is viewed at.
pub type BlameKey = (String, Option<String>);

#[derive(Debug)]
pub enum GitJob {
    Blame { epoch: u64, key: BlameKey },
    Candidates { epoch: u64, key: CandidateQueryKey },
}

#[derive(Debug)]
pub enum GitJobResult {
    Blame {
        epoch: u64,
        key: BlameKey,
        lines: Vec<BlameLine>,
    },
    Candidates {
        epoch: u64,
        key: CandidateQueryKey,
        commits: Option<Vec<CommitSummary>>,
    },
    History {
        epoch: u64,
        commits: Option<Vec<CommitSummary>>,
        /// True when `commits` is the whole history rather than its newest part.
        complete: bool,
    },
}

/// Load enough of the commit history to include `hash`: the newest
/// `EXTENDED_HISTORY_LIMIT` commits if it is among them, otherwise everything. Returns the
/// commits and whether they are the whole history.
pub fn fetch_history_containing(
    repo: &GitRepo,
    hash: &str,
) -> Result<(Vec<CommitSummary>, bool), GitError> {
    let recent = repo.get_commit_history_brief(Some(EXTENDED_HISTORY_LIMIT))?;
    if recent.len() < EXTENDED_HISTORY_LIMIT || recent.iter().any(|c| c.matches_hash(hash)) {
        let complete = recent.len() < EXTENDED_HISTORY_LIMIT;
        return Ok((
            recent.into_iter().map(CommitSummary::from).collect(),
            complete,
        ));
    }
    let all = repo.get_commit_history_brief(None)?;
    Ok((all.into_iter().map(CommitSummary::from).collect(), true))
}

/// Run the history query described by `key`.
pub fn fetch_candidate_commits(
    repo: &GitRepo,
    key: &CandidateQueryKey,
) -> Result<Vec<CommitSummary>, GitError> {
    let limit = Some(CANDIDATE_COMMIT_LIMIT);
    let commits = match key {
        CandidateQueryKey::Commit { .. } => repo.get_commit_history(limit),
        CandidateQueryKey::File { file_path, .. } => repo.get_file_commits(file_path, limit),
        CandidateQueryKey::LineRange {
            file_path,
            start_line,
            end_line,
            ..
        } => repo.get_line_commits(file_path, *start_line, *end_line, limit),
    }?;
    Ok(commits.into_iter().map(CommitSummary::from).collect())
}

/// Handle to the worker thread. Dropping it closes the job channel, which stops the thread.
pub struct GitWorker {
    repo: GitRepo,
    jobs: Sender<GitJob>,
    results: Receiver<GitJobResult>,
    result_sender: Sender<GitJobResult>,
}

impl GitWorker {
    pub fn spawn(repo: GitRepo) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<GitJob>();
        let (result_tx, result_rx) = mpsc::channel::<GitJobResult>();
        let result_sender = result_tx.clone();
        let worker_repo = repo.clone();

        std::thread::spawn(move || {
            let repo = worker_repo;
            while let Ok(first) = job_rx.recv() {
                // Requests pile up while a query runs. Only the newest of each kind still
                // matters, so older ones are dropped rather than run.
                let mut blame = None;
                let mut candidates = None;
                let mut next = Some(first);
                while let Some(job) = next {
                    match job {
                        GitJob::Blame { .. } => blame = Some(job),
                        GitJob::Candidates { .. } => candidates = Some(job),
                    }
                    next = job_rx.try_recv().ok();
                }

                for job in [blame, candidates].into_iter().flatten() {
                    let result = match job {
                        GitJob::Blame { epoch, key } => {
                            let lines = repo
                                .get_blame_at_commit(key.1.as_deref(), &key.0, None, None)
                                .unwrap_or_default();
                            GitJobResult::Blame { epoch, key, lines }
                        }
                        GitJob::Candidates { epoch, key } => {
                            let commits = fetch_candidate_commits(&repo, &key).ok();
                            GitJobResult::Candidates {
                                epoch,
                                key,
                                commits,
                            }
                        }
                    };
                    if result_tx.send(result).is_err() {
                        return;
                    }
                }
            }
        });

        Self {
            repo,
            jobs: job_tx,
            results: result_rx,
            result_sender,
        }
    }

    /// Load the commit history out to `hash` on a thread of its own. Listing a long
    /// history takes far longer than the queries that follow the cursor, so it must not
    /// hold them up in the shared queue.
    pub fn load_history(&self, epoch: u64, hash: String) {
        let repo = self.repo.clone();
        let results = self.result_sender.clone();
        std::thread::spawn(move || {
            let result = match fetch_history_containing(&repo, &hash) {
                Ok((commits, complete)) => GitJobResult::History {
                    epoch,
                    commits: Some(commits),
                    complete,
                },
                Err(_) => GitJobResult::History {
                    epoch,
                    commits: None,
                    complete: false,
                },
            };
            let _ = results.send(result);
        });
    }

    pub fn submit(&self, job: GitJob) {
        let _ = self.jobs.send(job);
    }

    pub fn try_recv(&self) -> Option<GitJobResult> {
        match self.results.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => None,
        }
    }
}
