use git_tardis::app::{ActivePanel, AppState, NavigationMode, SidebarView};
use git_tardis::ui::Action;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {:?} failed", args);
}

/// notes.txt has three lines. Commit 1 creates it, Commit 2 changes line 2 and Commit 3
/// changes line 3, so each line has a different history.
fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q"]);
    git(p, &["config", "user.name", "Test User"]);
    git(p, &["config", "user.email", "test@example.com"]);
    for (content, message) in [
        ("one\ntwo\nthree\n", "Commit 1"),
        ("one\nTWO\nthree\n", "Commit 2"),
        ("one\nTWO\nTHREE\n", "Commit 3"),
    ] {
        std::fs::write(p.join("notes.txt"), content).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-q", "-m", message]);
    }
    dir
}

fn background_app(dir: &TempDir) -> AppState {
    let mut app = AppState::new(dir.path().to_path_buf());
    app.enable_background_git();
    app.reload_repo_data();
    app.open_file_at_line("notes.txt", Some(1));
    app.active_panel = ActivePanel::CodeViewer;
    app
}

/// Poll until `done` holds, failing the test if it takes unreasonably long.
fn wait_for(app: &mut AppState, what: &str, done: impl Fn(&AppState) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.poll_background();
        if done(app) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {}", what);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn candidate_messages(app: &AppState) -> Vec<String> {
    app.candidate_commits
        .iter()
        .map(|c| c.message.clone())
        .collect()
}

#[test]
fn test_poll_background_is_inert_without_background_git() {
    let dir = setup_repo();
    let mut app = AppState::new(dir.path().to_path_buf());
    app.reload_repo_data();
    app.open_file_at_line("notes.txt", Some(2));

    // Synchronous mode resolves blame before returning.
    assert_eq!(
        app.current_line_blame.as_ref().map(|b| b.summary.as_str()),
        Some("Commit 2")
    );
    assert!(!app.has_pending_background());

    // The only thing that can still arrive is the git version looked up at startup.
    wait_for(&mut app, "git version", |a| a.git_version.is_some());
    assert!(!app.poll_background());
}

#[test]
fn test_blame_is_filled_in_by_the_worker() {
    let dir = setup_repo();
    let mut app = background_app(&dir);

    assert_eq!(
        app.blame_subprocess_count, 0,
        "blame must not run on the calling thread"
    );
    assert!(app.current_line_blame.is_none());
    assert!(app.has_pending_background());

    wait_for(&mut app, "blame", |a| a.current_line_blame.is_some());
    assert_eq!(app.current_line_blame.as_ref().unwrap().summary, "Commit 1");

    // Once the file is blamed, moving the cursor is answered from the cache immediately.
    app.move_selection_down();
    assert_eq!(app.current_line_blame.as_ref().unwrap().summary, "Commit 2");
    app.move_selection_down();
    assert_eq!(app.current_line_blame.as_ref().unwrap().summary, "Commit 3");
    assert_eq!(app.blame_subprocess_count, 0);
}

#[test]
fn test_candidates_are_deferred_then_applied() {
    let dir = setup_repo();
    let mut app = background_app(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::Line);
    wait_for(&mut app, "line 1 candidates", |a| {
        candidate_messages(a) == ["Commit 1"]
    });

    app.move_selection_down();
    assert_eq!(
        candidate_messages(&app),
        ["Commit 1"],
        "the query for the new line must not run on the calling thread"
    );
    assert!(app.has_pending_background());

    wait_for(&mut app, "line 2 candidates", |a| {
        candidate_messages(a) == ["Commit 2", "Commit 1"]
    });
    assert!(!app.has_pending_background());
}

#[test]
fn test_poll_reports_a_redraw_when_candidates_arrive() {
    let dir = setup_repo();
    let mut app = background_app(&dir);
    wait_for(&mut app, "blame", |a| a.current_line_blame.is_some());
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::Line);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut redraw_requested = false;
    while app.has_pending_background() {
        redraw_requested |= app.poll_background();
        assert!(
            Instant::now() < deadline,
            "timed out waiting for candidates"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(redraw_requested);
    assert_eq!(candidate_messages(&app), ["Commit 1"]);
}

#[test]
fn test_rapid_cursor_moves_only_apply_the_final_position() {
    let dir = setup_repo();
    let mut app = background_app(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::Line);
    wait_for(&mut app, "line 1 candidates", |a| {
        candidate_messages(a) == ["Commit 1"]
    });

    // Two quick moves, well inside the debounce window.
    app.move_selection_down();
    app.poll_background();
    app.move_selection_down();
    assert_eq!(app.cursor_line, 3);

    wait_for(&mut app, "pending work to finish", |a| {
        !a.has_pending_background()
    });
    assert_eq!(candidate_messages(&app), ["Commit 3", "Commit 1"]);
    assert!(
        !app.candidate_commits_cache.keys().any(|k| matches!(
            k,
            git_tardis::app::CandidateQueryKey::LineRange { start_line: 2, .. }
        )),
        "the line passed over inside the debounce window should not have been queried"
    );
}

#[test]
fn test_jump_right_after_moving_uses_the_new_line() {
    let dir = setup_repo();
    let mut app = background_app(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::Line);
    wait_for(&mut app, "line 1 candidates", |a| {
        candidate_messages(a) == ["Commit 1"]
    });

    // Move to line 3 and jump before the debounce has elapsed. HEAD (Commit 3) is the
    // newest change to line 3, so the previous version of that line is Commit 1.
    app.move_selection_down();
    app.move_selection_down();
    let moved_at = Instant::now();
    app.dispatch_action(Action::JumpPrevLine);
    assert!(
        moved_at.elapsed() < Duration::from_secs(5),
        "jump should not wait on the worker"
    );

    assert_eq!(candidate_messages(&app), ["Commit 3", "Commit 1"]);
    assert_eq!(app.code_lines, vec!["one", "two", "three"]);
    assert!(
        app.status_message.contains("Commit 1"),
        "{}",
        app.status_message
    );
}

#[test]
fn test_reload_discards_results_from_before_the_reload() {
    let dir = setup_repo();
    let mut app = background_app(&dir);
    wait_for(&mut app, "blame", |a| a.current_line_blame.is_some());

    // Rewrite history so the first line now belongs to a new commit, then reload.
    std::fs::write(dir.path().join("notes.txt"), "ONE\nTWO\nTHREE\n").unwrap();
    git(dir.path(), &["commit", "-q", "-am", "Commit 4"]);
    app.reload_repo_data();
    assert!(
        app.current_line_blame.is_none(),
        "blame cached before the reload must not be reused"
    );

    wait_for(&mut app, "fresh blame", |a| a.current_line_blame.is_some());
    assert_eq!(app.current_line_blame.as_ref().unwrap().summary, "Commit 4");
}
