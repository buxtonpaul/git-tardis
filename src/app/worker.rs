//! Background thread for git queries that follow the cursor, so key handling never waits on
//! a subprocess.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use super::{CandidateQueryKey, CommitSummary};
use crate::git::{BlameLine, GitError, GitRepo};

/// Commit limit applied to candidate history queries.
pub const CANDIDATE_COMMIT_LIMIT: usize = 200;

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
    jobs: Sender<GitJob>,
    results: Receiver<GitJobResult>,
}

impl GitWorker {
    pub fn spawn(repo: GitRepo) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<GitJob>();
        let (result_tx, result_rx) = mpsc::channel::<GitJobResult>();

        std::thread::spawn(move || {
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
            jobs: job_tx,
            results: result_rx,
        }
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
