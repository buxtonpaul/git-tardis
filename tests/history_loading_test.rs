use git_tardis::app::{AppState, NavigationMode, SidebarView};
use git_tardis::git::GitRepo;
use git_tardis::ui::Action;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const COMMITS: usize = 1200;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A repository with `COMMITS` commits to one file, written with `git fast-import` so that
/// building it takes a fraction of a second. Commit N has the subject "Commit N".
fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "user.name", "Test User"]);
    git(p, &["config", "user.email", "test@example.com"]);

    let mut stream = String::new();
    for n in 1..=COMMITS {
        let message = format!("Commit {}\n", n);
        let content = format!("value {}\n", n);
        stream.push_str("commit refs/heads/main\n");
        stream.push_str(&format!("mark :{}\n", n));
        stream.push_str(&format!(
            "committer Test User <test@example.com> {} +0000\n",
            1_600_000_000 + n
        ));
        stream.push_str(&format!("data {}\n{}", message.len(), message));
        if n > 1 {
            stream.push_str(&format!("from :{}\n", n - 1));
        }
        stream.push_str(&format!(
            "M 100644 inline data.txt\ndata {}\n{}\n",
            content.len(),
            content
        ));
    }

    let mut child = Command::new("git")
        .current_dir(p)
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stream.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    git(p, &["reset", "-q", "--hard", "main"]);
    dir
}

/// Hash of the commit with subject "Commit N".
fn hash_of(dir: &TempDir, n: usize) -> String {
    let repo = GitRepo::open(dir.path()).unwrap();
    let all = repo.get_commit_history_brief(None).unwrap();
    assert_eq!(all.len(), COMMITS);
    all[COMMITS - n].hash.clone()
}

fn open_app(dir: &TempDir, background: bool) -> AppState {
    let mut app = AppState::new(dir.path().to_path_buf());
    if background {
        app.enable_background_git();
    }
    app.reload_repo_data();
    app.open_file_at_line("data.txt", Some(1));
    app.set_sidebar_view(SidebarView::CommitTimeline);
    assert_eq!(app.commits.len(), 200);
    app
}

fn settle(app: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        app.poll_background();
        if !app.has_pending_background() && !app.has_pending_jump() {
            return;
        }
        assert!(Instant::now() < deadline, "background work did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn selected_message(app: &AppState) -> String {
    app.display_commit_at(app.commit_selected)
        .map(|c| c.message)
        .unwrap_or_default()
}

#[test]
fn test_brief_history_matches_full_history() {
    let dir = setup_repo();
    let repo = GitRepo::open(dir.path()).unwrap();

    let full = repo.get_commit_history(Some(50)).unwrap();
    let brief = repo.get_commit_history_brief(Some(50)).unwrap();

    assert_eq!(brief.len(), full.len());
    for (b, f) in brief.iter().zip(&full) {
        assert_eq!(b.hash, f.hash);
        assert_eq!(b.short_hash, f.short_hash);
        assert_eq!(b.summary, f.summary);
        assert!(b.author.is_empty() && b.body.is_empty());
    }
}

#[test]
fn test_synchronous_mode_loads_history_before_returning() {
    let dir = setup_repo();
    let mut app = open_app(&dir, false);

    app.update_state_for_commit_hash(hash_of(&dir, 700));
    assert_eq!(app.commits.len(), 1000, "target is within the newest 1000");
    assert_eq!(selected_message(&app), "Commit 700");

    app.update_state_for_commit_hash(hash_of(&dir, 5));
    assert_eq!(app.commits.len(), COMMITS, "target needs the whole history");
    assert_eq!(selected_message(&app), "Commit 5");
}

#[test]
fn test_old_commit_history_arrives_in_the_background() {
    let dir = setup_repo();
    let mut app = open_app(&dir, true);
    settle(&mut app);

    app.update_state_for_commit_hash(hash_of(&dir, 5));

    // The call returns with the short list still in place.
    assert_eq!(app.commits.len(), 200);
    assert!(app.has_pending_background());

    settle(&mut app);
    assert_eq!(app.commits.len(), COMMITS);
    assert_eq!(selected_message(&app), "Commit 5");
}

#[test]
fn test_commit_within_recent_history_loads_only_that_much() {
    let dir = setup_repo();
    let mut app = open_app(&dir, true);
    settle(&mut app);

    app.update_state_for_commit_hash(hash_of(&dir, 700));
    settle(&mut app);

    assert_eq!(app.commits.len(), 1000);
    assert_eq!(selected_message(&app), "Commit 700");

    // Going further back afterwards extends the list again.
    app.update_state_for_commit_hash(hash_of(&dir, 5));
    settle(&mut app);
    assert_eq!(app.commits.len(), COMMITS);
    assert_eq!(selected_message(&app), "Commit 5");
}

#[test]
fn test_commit_jump_waits_for_history_and_steps_from_the_right_row() {
    let dir = setup_repo();
    let mut app = open_app(&dir, true);
    app.set_navigation_mode(NavigationMode::Commit);
    settle(&mut app);

    app.update_state_for_commit_hash(hash_of(&dir, 50));
    app.load_currently_selected_file();
    assert_eq!(app.commits.len(), 200);

    // The selected commit's row is not loaded yet, so stepping to its neighbour waits.
    app.dispatch_action(Action::JumpPrevCommit);
    assert!(app.has_pending_jump());
    assert_eq!(app.selected_commit_hash, Some(hash_of(&dir, 50)));
    assert_eq!(app.status_message, "Loading commit history...");

    settle(&mut app);
    assert_eq!(app.selected_commit_hash, Some(hash_of(&dir, 49)));
    assert_eq!(selected_message(&app), "Commit 49");
    assert_eq!(app.code_lines, vec!["value 49"]);
}

#[test]
fn test_reload_discards_a_history_load_in_progress() {
    let dir = setup_repo();
    let mut app = open_app(&dir, true);
    settle(&mut app);

    app.update_state_for_commit_hash(hash_of(&dir, 5));
    app.reload_repo_data();

    // Whatever the old load returns must not replace the freshly reloaded list.
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        app.poll_background();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(app.commits.len(), 200);
}
