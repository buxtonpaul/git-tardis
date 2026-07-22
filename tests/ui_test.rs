use git_tardis::app::{ActivePanel, AppState, NavigationMode, SidebarView};
use git_tardis::config::Config;
use git_tardis::ui::{
    Action, KeyDispatcher, KeyStroke, KeymapRegistry, Scope, render,
};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use std::path::PathBuf;

#[test]
fn test_split_panel_rendering_and_borders() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["src/main.rs".to_string(), "Cargo.toml".to_string()];
    app.modified_files = vec!["src/main.rs (M)".to_string()];
    app.commits = vec![("a1b2c3d".to_string(), "feat: initial commit".to_string())];
    app.code_lines = vec!["fn main() {".to_string(), "    println!(\"Hello\");".to_string(), "}".to_string()];

    // 1. Initial State: Sidebar active
    terminal.draw(|f| render(f, &app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let dbg_str = format!("{:?}", buffer);

    assert!(dbg_str.contains("Git-tardis TUI"));
    assert!(dbg_str.contains("1: Explorer"));
    assert!(dbg_str.contains("2: Modified Files"));
    assert!(dbg_str.contains("3: Commit Timeline"));
    assert!(dbg_str.contains("Sidebar [1: Explorer]"));
    assert!(dbg_str.contains("Code Viewer - Mode: [FILE Mode]"));

    // Check Cyan border for active Sidebar (top-left border cell at (0, 3))
    assert_eq!(buffer[(0, 3)].fg, Color::Cyan);
    // Check DarkGray border for inactive CodeViewer (top-left border cell at (32, 3))
    assert_eq!(buffer[(32, 3)].fg, Color::DarkGray);

    // 2. Toggle focus to CodeViewer
    app.toggle_panel_focus();
    terminal.draw(|f| render(f, &app)).unwrap();
    let buffer2 = terminal.backend().buffer().clone();

    // Now Sidebar is DarkGray, CodeViewer is Cyan
    assert_eq!(buffer2[(0, 3)].fg, Color::DarkGray);
    assert_eq!(buffer2[(32, 3)].fg, Color::Cyan);
}

#[test]
fn test_sidebar_tab_views_rendering() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["file1.rs".to_string()];
    app.modified_files = vec!["mod1.rs (M)".to_string()];
    app.commits = vec![("1234567".to_string(), "commit message".to_string())];

    // Tab 1: Explorer
    app.set_sidebar_view(SidebarView::FileExplorer);
    terminal.draw(|f| render(f, &app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg1.contains("file1.rs"));

    // Tab 2: Modified Files
    app.set_sidebar_view(SidebarView::ModifiedFiles);
    terminal.draw(|f| render(f, &app)).unwrap();
    let dbg2 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg2.contains("mod1.rs (M)"));

    // Tab 3: Commit Timeline
    app.set_sidebar_view(SidebarView::CommitTimeline);
    terminal.draw(|f| render(f, &app)).unwrap();
    let dbg3 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg3.contains("1234567 commit message"));
}

#[test]
fn test_keymap_scope_fallback_and_dispatcher() {
    let registry = KeymapRegistry::new();
    let mut dispatcher = KeyDispatcher::new(registry);

    let mut app = AppState::new(PathBuf::from("."));
    assert_eq!(app.active_panel, ActivePanel::Sidebar);

    // Press 'j' in Sidebar scope -> MoveDown
    let action_j = dispatcher.handle_key(KeyStroke::Char('j'), app.active_scope());
    assert_eq!(action_j, Some(Action::MoveDown));

    // Press Tab in Sidebar scope -> Fallback to Global -> ToggleFocus
    let action_tab = dispatcher.handle_key(KeyStroke::Tab, app.active_scope());
    assert_eq!(action_tab, Some(Action::ToggleFocus));
    if let Some(act) = action_tab {
        app.dispatch_action(act);
    }
    assert_eq!(app.active_panel, ActivePanel::CodeViewer);

    // Press '1' in CodeViewer scope -> Fallback to Global -> SetSidebarView(1)
    let action_1 = dispatcher.handle_key(KeyStroke::Char('1'), app.active_scope());
    assert_eq!(action_1, Some(Action::SetSidebarView(1)));

    // Press 'm' in CodeViewer scope -> Fallback to Global -> CycleNavMode
    let action_m = dispatcher.handle_key(KeyStroke::Char('m'), app.active_scope());
    assert_eq!(action_m, Some(Action::CycleNavMode));
    if let Some(act) = action_m {
        app.dispatch_action(act);
    }
    assert_eq!(app.nav_mode, NavigationMode::Function);
}

#[test]
fn test_multi_key_sequence_matching() {
    let registry = KeymapRegistry::new();
    let mut dispatcher = KeyDispatcher::new(registry);

    // Sequence ]f in CodeViewer scope
    let act1 = dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
    assert_eq!(act1, None); // Ambiguous

    let act2 = dispatcher.handle_key(KeyStroke::Char('f'), Scope::CodeViewer);
    assert_eq!(act2, Some(Action::JumpNextFunction));

    // Sequence ]l in CodeViewer scope
    dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
    let act3 = dispatcher.handle_key(KeyStroke::Char('l'), Scope::CodeViewer);
    assert_eq!(act3, Some(Action::JumpNextLine));

    // Sequence ]m in CodeViewer scope
    dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
    let act4 = dispatcher.handle_key(KeyStroke::Char('m'), Scope::CodeViewer);
    assert_eq!(act4, Some(Action::JumpNextFile));

    // Sequence [f in CodeViewer scope
    dispatcher.handle_key(KeyStroke::Char('['), Scope::CodeViewer);
    let act5 = dispatcher.handle_key(KeyStroke::Char('f'), Scope::CodeViewer);
    assert_eq!(act5, Some(Action::JumpPrevFunction));
}

#[test]
fn test_toml_custom_keymap_config_loading() {
    let toml_str = r#"
    [keymaps.global]
    quit = ["q", "<Esc>", "<C-c>"]

    [keymaps.code_viewer]
    jump_next_function = ["]f", "<C-f>"]
    "#;

    let config = Config::from_toml(toml_str).unwrap();
    let mut registry = KeymapRegistry::new();
    if let Some(km) = &config.keymaps {
        registry.apply_config(km);
    }

    let mut dispatcher = KeyDispatcher::new(registry);

    let act_ctrl_c = dispatcher.handle_key(KeyStroke::Ctrl('c'), Scope::Sidebar);
    assert_eq!(act_ctrl_c, Some(Action::Quit));

    let act_ctrl_f = dispatcher.handle_key(KeyStroke::Ctrl('f'), Scope::CodeViewer);
    assert_eq!(act_ctrl_f, Some(Action::JumpNextFunction));
}

#[test]
fn test_sidebar_file_selection_loads_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file1_path = temp_dir.path().join("file1.rs");
    std::fs::write(&file1_path, "println!(\"File 1 content\");\n").unwrap();

    let mut app = AppState::new(temp_dir.path().to_path_buf());
    app.files = vec!["file1.rs".to_string()];

    let registry = KeymapRegistry::new();
    let mut dispatcher = KeyDispatcher::new(registry);

    // Press 'l' or Enter in Sidebar scope
    let act = dispatcher.handle_key(KeyStroke::Char('l'), app.active_scope());
    assert_eq!(act, Some(Action::Select));

    if let Some(a) = act {
        app.dispatch_action(a);
    }

    assert_eq!(app.code_lines, vec!["println!(\"File 1 content\");"]);
    assert!(app.status_message.contains("Loaded file: file1.rs"));
}
