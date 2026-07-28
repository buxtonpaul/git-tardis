use clap::Parser;
use git_tardis::app::{ActivePanel, AppState};
use git_tardis::cli::CliArgs;
use git_tardis::ui::parse_color;
use ratatui::style::Color;
use std::fs;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let main_file = src_dir.join("main.rs");
    let content = (1..=50)
        .map(|i| format!("// Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&main_file, content).unwrap();

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

#[test]
fn test_cli_args_file_and_line_focus() {
    let repo_dir = setup_test_repo();

    let args = CliArgs::try_parse_from([
        "git-tardis",
        repo_dir.path().to_str().unwrap(),
        "--file",
        "src/main.rs",
        "--line",
        "25",
    ])
    .unwrap();

    let mut app = AppState::new(args.path);
    app.reload_repo_data();

    if let Some(target_file) = args.file {
        app.open_file_at_line(target_file, args.line);
    }

    assert_eq!(app.active_file, Some("src/main.rs".to_string()));
    assert_eq!(app.active_panel, ActivePanel::CodeViewer);
    assert_eq!(app.cursor_line, 25);
}

#[test]
fn test_cli_theme_color_parsing_and_app_state() {
    let bg_color = parse_color("#1e1e2e");
    let fg_color = parse_color("cyan");

    assert_eq!(bg_color, Some(Color::Rgb(30, 30, 46)));
    assert_eq!(fg_color, Some(Color::Cyan));

    let repo_dir = setup_test_repo();
    let mut app = AppState::new(repo_dir.path().to_path_buf());
    app.set_theme_colors(bg_color, fg_color);

    assert_eq!(app.theme_bg, Some(Color::Rgb(30, 30, 46)));
    assert_eq!(app.theme_fg, Some(Color::Cyan));
}

#[test]
fn test_worktree_file_and_line_focus() {
    let repo_dir = setup_test_repo();
    let wt_dir = tempfile::tempdir().unwrap();
    let wt_path = wt_dir.path().join("worktree1");

    let status = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "feature-worktree",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(repo_dir.path())
        .status()
        .unwrap();

    assert!(status.success());

    let abs_wt_file = wt_path.join("src").join("main.rs");

    let mut app = AppState::new(wt_path.clone());
    app.reload_repo_data();

    app.open_file_at_line(&abs_wt_file, Some(15));

    assert_eq!(app.active_file, Some("src/main.rs".to_string()));
    assert_eq!(app.active_panel, ActivePanel::CodeViewer);
    assert_eq!(app.cursor_line, 15);
}
