use git_tardis::app::{AppState, NavigationMode, SidebarView};
use git_tardis::git::GitRepo;
use git_tardis::timeline::{JumpDirection, JumpScope, TimelineJumpRequest, TimelineNavigator};
use git_tardis::treesitter::GrammarRegistry;
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(status.status.success(), "git {:?} failed", args);
}

/// Three commits: main.rs v1, then other.rs added, then main.rs v2.
fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q"]);
    git(p, &["config", "user.name", "Test User"]);
    git(p, &["config", "user.email", "test@example.com"]);

    std::fs::write(p.join("main.rs"), "fn main() {\n    one();\n}\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "Commit 1"]);

    std::fs::write(p.join("other.rs"), "fn other() {}\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "Commit 2"]);

    std::fs::write(p.join("main.rs"), "fn main() {\n    two();\n}\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "Commit 3"]);
    dir
}

fn open_main_rs(dir: &TempDir) -> AppState {
    let mut app = AppState::new(dir.path().to_path_buf());
    app.reload_repo_data();
    app.open_file_at_line("main.rs", Some(2));
    app
}

#[test]
fn test_jump_seeds_file_content_cache() {
    let dir = setup_repo();
    let mut app = open_main_rs(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::File);

    app.dispatch_action(Action::JumpPrevFile);

    let hash = app
        .selected_commit_hash
        .clone()
        .expect("should be time travelling");
    assert_eq!(app.code_lines, vec!["fn main() {", "    one();", "}"]);
    assert_eq!(
        app.file_content_cache
            .get(&("main.rs".to_string(), hash))
            .map(|lines| lines.len()),
        Some(3)
    );
}

#[test]
fn test_file_list_is_deferred_until_explorer_is_shown() {
    let dir = setup_repo();
    let mut app = open_main_rs(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::File);
    assert_eq!(app.files, vec!["main.rs", "other.rs"]);

    // Jump back to Commit 1, where other.rs does not exist yet.
    app.dispatch_action(Action::JumpPrevFile);
    assert!(app.selected_commit_hash.is_some());
    assert_eq!(
        app.files,
        vec!["main.rs", "other.rs"],
        "tree listing should not be fetched while the explorer is hidden"
    );

    app.set_sidebar_view(SidebarView::FileExplorer);
    assert_eq!(app.files, vec!["main.rs"]);
}

#[test]
fn test_explorer_shows_commit_files_when_view_is_switched_directly() {
    let dir = setup_repo();
    let mut app = open_main_rs(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::File);
    app.dispatch_action(Action::JumpPrevFile);

    // Switch the view without going through set_sidebar_view; rendering must still catch up.
    app.sidebar_view = SidebarView::FileExplorer;
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();

    assert_eq!(app.files, vec!["main.rs"]);
    let screen = format!("{:?}", terminal.backend().buffer());
    assert!(!screen.contains("other.rs"));
}

#[test]
fn test_file_list_is_loaded_immediately_when_explorer_is_visible() {
    let dir = setup_repo();
    let mut app = open_main_rs(&dir);
    app.sidebar_view = SidebarView::FileExplorer;
    app.set_navigation_mode(NavigationMode::File);

    app.dispatch_action(Action::JumpPrevFile);

    assert_eq!(app.files, vec!["main.rs"]);
}

#[test]
fn test_reset_time_travel_restores_working_tree_files_after_deferred_jump() {
    let dir = setup_repo();
    let mut app = open_main_rs(&dir);
    app.sidebar_view = SidebarView::CommitTimeline;
    app.set_navigation_mode(NavigationMode::File);
    app.dispatch_action(Action::JumpPrevFile);

    app.reset_time_travel();
    app.set_sidebar_view(SidebarView::FileExplorer);

    assert_eq!(app.selected_commit_hash, None);
    assert_eq!(app.files, vec!["main.rs", "other.rs"]);
}

#[test]
fn test_navigator_with_shared_registry_finds_function_history() {
    let dir = setup_repo();
    let repo = GitRepo::open(dir.path()).unwrap();
    let registry = GrammarRegistry::new();
    let source: Vec<String> = ["fn main() {", "    two();", "}"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let nav = TimelineNavigator::with_registry(&registry);
    let res = nav
        .jump(TimelineJumpRequest {
            repo_path: repo.work_dir(),
            repo: Some(&repo),
            file_path: "main.rs",
            source_lines: &source,
            cursor_line: 2,
            current_commit_hash: None,
            head_commit_hash: None,
            scope: JumpScope::Function,
            direction: JumpDirection::Previous,
            cached_commits: None,
            is_dirty: false,
        })
        .unwrap()
        .unwrap();

    assert_eq!(res.commit_summary, "Commit 1");
    assert_eq!(res.line_range, Some((1, 3)));
}
