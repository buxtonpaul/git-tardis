use std::path::PathBuf;

use git_tardis::app::AppState;
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_keybinds_help_toggle_and_dismissal() {
    let mut app = AppState::new(PathBuf::from("."));
    assert!(!app.show_help);

    // Dispatch Action::ToggleHelp (triggered by '?')
    app.dispatch_action(Action::ToggleHelp);
    assert!(app.show_help);
    assert!(app
        .status_message
        .contains("Opened keybindings help overlay"));

    // Pressing Esc or 'q' (Action::Quit) dismisses the help overlay without quitting app
    app.dispatch_action(Action::Quit);
    assert!(!app.show_help);
    assert!(app.running);
    assert!(app
        .status_message
        .contains("Closed keybindings help overlay"));

    // Toggling help again and pressing '?' again toggles it off
    app.dispatch_action(Action::ToggleHelp);
    assert!(app.show_help);

    app.dispatch_action(Action::ToggleHelp);
    assert!(!app.show_help);
}

#[test]
fn test_keybinds_help_screen_rendering() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.dispatch_action(Action::ToggleHelp);
    assert!(app.show_help);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let dbg = format!("{:?}", buffer);

    // Verify keybindings popup title and contents
    assert!(dbg.contains("Keybindings Help (?)"));
    assert!(dbg.contains("Global Shortcuts"));
    assert!(dbg.contains("Sidebar Navigation"));
    assert!(dbg.contains("Code Viewer & Time Travel"));
    assert!(dbg.contains("Toggle Keybindings Help Screen"));
    assert!(dbg.contains("Switch Focus between Sidebar and Code Viewer"));
    assert!(dbg.contains("Toggle File Viewer Mode"));
    assert!(dbg.contains("Press '?' or 'Esc' to close this help window"));
}
