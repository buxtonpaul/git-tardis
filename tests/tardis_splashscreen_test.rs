use std::path::PathBuf;

use git_tardis::app::AppState;
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_splashscreen_toggle_and_dismissal() {
    let mut app = AppState::new(PathBuf::from("."));
    assert!(!app.show_splashscreen);

    // Dispatch Action::ToggleSplashscreen (triggered by 'S')
    app.dispatch_action(Action::ToggleSplashscreen);
    assert!(app.show_splashscreen);
    assert!(app.status_message.contains("Opened TARDIS splashscreen"));

    // Pressing Esc or 'q' (Action::Quit) dismisses the splashscreen without quitting app
    app.dispatch_action(Action::Quit);
    assert!(!app.show_splashscreen);
    assert!(app.running);
    assert!(app.status_message.contains("Closed TARDIS splashscreen"));

    // Toggle splashscreen again and press 'S' to toggle off
    app.dispatch_action(Action::ToggleSplashscreen);
    assert!(app.show_splashscreen);

    app.dispatch_action(Action::ToggleSplashscreen);
    assert!(!app.show_splashscreen);
}

#[test]
fn test_splashscreen_rendering_and_metadata() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.git_version = Some("git version 2.45.0".to_string());
    app.dispatch_action(Action::ToggleSplashscreen);
    assert!(app.show_splashscreen);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let dbg = format!("{:?}", buffer);

    // Verify TARDIS ASCII art components
    assert!(dbg.contains("POLICE  BOX"));

    // Verify metadata details
    assert!(dbg.contains("Git-tardis"));
    assert!(dbg.contains("Code Time-Travel Navigator"));
    assert!(dbg.contains("Version: v"));
    assert!(dbg.contains("System Git: git version 2.45.0"));
    assert!(dbg.contains("Copyright (c) 2026 Paul Buxton"));
}

#[test]
fn test_code_viewer_default_splashscreen_rendering() {
    let backend = TestBackend::new(100, 35);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.code_lines.clear(); // Ensure no file loaded

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let dbg = format!("{:?}", buffer);

    // Verify TARDIS ASCII art and metadata render in Code Viewer by default when no file is loaded
    assert!(dbg.contains("POLICE  BOX"));
    assert!(dbg.contains("Git-tardis"));
    assert!(dbg.contains("Copyright (c) 2026 Paul Buxton"));
}

#[test]
fn test_git_version_arrives_without_blocking_startup() {
    let mut app = AppState::new(PathBuf::from("."));

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut reported_change = false;
    while app.git_version.is_none() {
        reported_change |= app.poll_background();
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for git version"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(reported_change, "arrival should request a redraw");
    assert!(app
        .git_version
        .as_deref()
        .unwrap()
        .starts_with("git version"));
    assert!(!app.poll_background(), "nothing further to report");
}

#[test]
fn test_explicit_git_version_is_not_overwritten() {
    let mut app = AppState::new(PathBuf::from("."));
    app.git_version = Some("git version 0.0.0-test".to_string());

    std::thread::sleep(std::time::Duration::from_millis(300));
    app.poll_background();

    assert_eq!(app.git_version.as_deref(), Some("git version 0.0.0-test"));
}
