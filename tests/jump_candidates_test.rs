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

fn source(a: u32, b: u32) -> String {
    format!(
        "fn alpha() {{\n    let a = {};\n}}\n\nfn beta() {{\n    let b = {};\n}}\n",
        a, b
    )
}

/// Two functions with different histories. `alpha` changes in Commits 2 and 4, `beta` in
/// Commit 3, so from HEAD (Commit 4) the previous change of `beta` is Commit 3, while the
/// previous change of `alpha` is Commit 2.
fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q"]);
    git(p, &["config", "user.name", "Test User"]);
    git(p, &["config", "user.email", "test@example.com"]);
    for (i, (a, b)) in [(1, 1), (2, 1), (2, 3), (4, 3)].iter().enumerate() {
        std::fs::write(p.join("f.rs"), source(*a, *b)).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-q", "-m", &format!("Commit {}", i + 1)]);
    }
    dir
}

fn settle(app: &mut AppState) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.poll_background();
        if !app.has_pending_background() && !app.has_pending_jump() {
            return;
        }
        assert!(Instant::now() < deadline, "background work did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Start in `alpha` so its history is the one loaded, then move into `beta` and jump.
fn jump_from_beta(view: SidebarView, mode: NavigationMode, background: bool) -> AppState {
    let dir = setup_repo();
    let mut app = AppState::new(dir.path().to_path_buf());
    if background {
        app.enable_background_git();
    }
    app.reload_repo_data();
    app.open_file_at_line("f.rs", Some(2));
    app.active_panel = ActivePanel::CodeViewer;
    app.set_sidebar_view(view);
    app.set_navigation_mode(mode);
    settle(&mut app);

    for _ in 0..4 {
        app.dispatch_action(Action::MoveDown);
    }
    assert_eq!(app.code_lines[app.cursor_line - 1], "    let b = 3;");

    app.dispatch_action(Action::JumpPrevAuto);
    settle(&mut app);
    app
}

#[test]
fn test_jump_uses_history_of_the_cursor_position_in_every_view() {
    for view in [
        SidebarView::FileExplorer,
        SidebarView::ModifiedFiles,
        SidebarView::CommitTimeline,
    ] {
        for mode in [NavigationMode::Function, NavigationMode::Line] {
            for background in [false, true] {
                let app = jump_from_beta(view, mode, background);
                assert!(
                    app.status_message.contains("Commit 3"),
                    "view={:?} mode={:?} background={}: {}",
                    view,
                    mode,
                    background,
                    app.status_message
                );
                assert_eq!(app.code_lines[5], "    let b = 3;");
                assert_eq!(app.code_lines[1], "    let a = 2;");
            }
        }
    }
}
