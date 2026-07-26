use git_tardis::app::{ActivePanel, AppState, CommitSummary, NavigationMode, SidebarView};
use git_tardis::config::Config;
use git_tardis::ui::{render, Action, KeyDispatcher, KeyStroke, KeymapRegistry, Scope};
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};
use std::path::PathBuf;

#[test]
fn test_split_panel_rendering_and_borders() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["src/main.rs".to_string(), "Cargo.toml".to_string()];
    app.modified_files = vec!["src/main.rs (M)".into()];
    app.commits = vec![("a1b2c3d", "feat: initial commit").into()];
    app.code_lines = vec![
        "fn main() {".to_string(),
        "    println!(\"Hello\");".to_string(),
        "}".to_string(),
    ];

    // 1. Initial State: Sidebar active
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let dbg_str = format!("{:?}", buffer);

    assert!(dbg_str.contains("Git-tardis TUI"));
    assert!(dbg_str.contains("1: Explorer"));
    assert!(dbg_str.contains("2: Dirty Files"));
    assert!(dbg_str.contains("3: Commit Timeline"));
    assert!(dbg_str.contains("Sidebar [1: Explorer]"));
    assert!(dbg_str.contains("Code Viewer - Mode: [COMMIT Mode]"));

    // Check Cyan border for active Sidebar (top-left border cell at (0, 3))
    assert_eq!(buffer[(0, 3)].fg, Color::Cyan);
    // Check DarkGray border for inactive CodeViewer (top-left border cell at (32, 3))
    assert_eq!(buffer[(32, 3)].fg, Color::DarkGray);

    // 2. Toggle focus to CodeViewer
    app.toggle_panel_focus();
    terminal.draw(|f| render(f, &mut app)).unwrap();
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
    app.dirty_files = vec!["mod1.rs (M)".into()];
    app.commits = vec![("1234567", "commit message").into()];

    // Tab 1: Explorer
    app.set_sidebar_view(SidebarView::FileExplorer);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg1.contains("file1.rs"));

    // Tab 2: Modified Files
    app.set_sidebar_view(SidebarView::ModifiedFiles);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg2 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg2.contains("mod1.rs (M)"));

    // Tab 3: Commit Timeline
    app.set_sidebar_view(SidebarView::CommitTimeline);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg3 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg3.contains("1234567 commit message"));

    // Tab 4: Target Candidates
    app.set_sidebar_view(SidebarView::TargetCandidates);
    app.candidate_commits = vec![("7654321", "candidate commit").into()];
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg4 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg4.contains("7654321 candidate commit"));
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
    assert_eq!(app.nav_mode, NavigationMode::File);
}

#[test]
fn test_immediate_navigation_key_matching() {
    let registry = KeymapRegistry::new();
    let mut dispatcher = KeyDispatcher::new(registry);

    // Single ] key in CodeViewer scope immediately matches JumpNextAuto
    let act1 = dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
    assert_eq!(act1, Some(Action::JumpNextAuto));

    // Single [ key in CodeViewer scope immediately matches JumpPrevAuto
    let act2 = dispatcher.handle_key(KeyStroke::Char('['), Scope::CodeViewer);
    assert_eq!(act2, Some(Action::JumpPrevAuto));
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

#[test]
fn test_commit_selection_updates_modified_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    // Helper to run git
    let run = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    std::fs::write(repo_path.join("file_a.txt"), "Content A").unwrap();
    run(&["add", "file_a.txt"]);
    run(&["commit", "-m", "First commit file_a"]);

    std::fs::write(repo_path.join("file_b.txt"), "Content B").unwrap();
    run(&["add", "file_b.txt"]);
    run(&["commit", "-m", "Second commit file_b"]);

    let repo = git_tardis::git::GitRepo::open(repo_path).unwrap();
    let history = repo.get_commit_history(Some(10)).unwrap();

    let mut app = AppState::new(repo_path.to_path_buf());
    app.commits = history.into_iter().map(CommitSummary::from).collect();

    // Highlight commit 0 (Second commit file_b)
    app.commit_selected = 0;
    app.update_modified_files_for_selected_commit();
    assert_eq!(app.modified_files.len(), 1);
    assert!(app.modified_files[0].path.contains("file_b.txt"));

    // Highlight commit 1 (First commit file_a)
    app.commit_selected = 1;
    app.update_modified_files_for_selected_commit();
    assert_eq!(app.modified_files.len(), 1);
    assert!(app.modified_files[0].path.contains("file_a.txt"));
}

#[test]
fn test_auto_update_code_viewer_on_file_explorer_navigation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file1_path = temp_dir.path().join("file1.rs");
    let file2_path = temp_dir.path().join("file2.rs");
    std::fs::write(&file1_path, "fn file1() {}\n").unwrap();
    std::fs::write(&file2_path, "fn file2() {}\n").unwrap();

    let mut app = AppState::new(temp_dir.path().to_path_buf());
    app.files = vec!["file1.rs".to_string(), "file2.rs".to_string()];
    app.load_currently_selected_file();
    assert_eq!(app.code_lines, vec!["fn file1() {}"]);

    // Move selection down (j) -> file2.rs should load automatically
    app.move_selection_down();
    assert_eq!(app.file_selected, 1);
    assert_eq!(app.code_lines, vec!["fn file2() {}"]);
    assert!(app.status_message.contains("Loaded file: file2.rs"));

    // Move selection up (k) -> file1.rs should load automatically
    app.move_selection_up();
    assert_eq!(app.file_selected, 0);
    assert_eq!(app.code_lines, vec!["fn file1() {}"]);
    assert!(app.status_message.contains("Loaded file: file1.rs"));
}

#[test]
fn test_auto_update_code_viewer_on_modified_files_navigation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    std::fs::write(repo_path.join("file_a.txt"), "Original A\n").unwrap();
    std::fs::write(repo_path.join("file_b.txt"), "Original B\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Initial commit"]);

    std::fs::write(repo_path.join("file_a.txt"), "Modified A\n").unwrap();
    std::fs::write(repo_path.join("file_b.txt"), "Modified B\n").unwrap();

    let mut app = AppState::new(repo_path.to_path_buf());
    app.set_sidebar_view(SidebarView::ModifiedFiles);
    app.dirty_files = vec!["file_a.txt (M)".into(), "file_b.txt (M)".into()];
    app.load_currently_selected_file();
    assert_eq!(app.code_lines, vec!["Modified A"]);

    // Move selection down -> file_b.txt should load automatically
    app.move_selection_down();
    assert_eq!(app.dirty_selected, 1);
    assert_eq!(app.code_lines, vec!["Modified B"]);

    // Move selection up -> file_a.txt should load automatically
    app.move_selection_up();
    assert_eq!(app.dirty_selected, 0);
    assert_eq!(app.code_lines, vec!["Modified A"]);
}

#[test]
fn test_historical_commit_modified_files_auto_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Test User"]);
    run(&["config", "user.email", "test@example.com"]);

    std::fs::write(repo_path.join("file1.txt"), "V1 File 1\n").unwrap();
    std::fs::write(repo_path.join("file2.txt"), "V1 File 2\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "First commit"]);

    let repo = git_tardis::git::GitRepo::open(repo_path).unwrap();
    let history = repo.get_commit_history(Some(1)).unwrap();
    let first_commit_hash = history[0].hash.clone();

    let mut app = AppState::new(repo_path.to_path_buf());
    app.commits = vec![(
        first_commit_hash[..7].to_string(),
        "First commit".to_string(),
    )
        .into()];

    // Select the commit from timeline (Action::Select on CommitTimeline)
    app.set_sidebar_view(SidebarView::CommitTimeline);
    app.dispatch_action(Action::Select);

    // Sidebar view is now ModifiedFiles for first_commit
    assert_eq!(app.sidebar_view, SidebarView::ModifiedFiles);
    assert_eq!(app.modified_files.len(), 2);
    // Auto-loaded first modified file
    assert_eq!(app.code_lines, vec!["V1 File 1"]);

    // Moving down in ModifiedFiles auto-loads second file at that commit
    app.move_selection_down();
    assert_eq!(app.modified_selected, 1);
    assert_eq!(app.code_lines, vec!["V1 File 2"]);
}

#[test]
fn test_code_viewer_syntax_highlighting_rendering() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["src/main.rs".to_string()];
    app.file_selected = 0;
    app.code_lines = vec![
        "fn main() {".to_string(),
        "    let msg = \"hello world\";".to_string(),
        "}".to_string(),
    ];

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();

    // Code viewer panel starts at x=32, y=3
    // Inner text starts at x=33, y=4
    // Line 1: "  1 > fn main() {"
    let mut line1_str = String::new();
    for x in 33..50 {
        line1_str.push_str(buffer[(x, 4)].symbol());
    }
    assert!(line1_str.contains("fn main()"));

    // Find x offset where "fn" is located
    let fn_pos = line1_str.find("fn").unwrap();
    let fn_x = 33 + fn_pos as u16;
    let f_cell = &buffer[(fn_x, 4)];
    let n_cell = &buffer[(fn_x + 1, 4)];
    assert_eq!(f_cell.fg, Color::Magenta);
    assert_eq!(n_cell.fg, Color::Magenta);

    // Find x offset where "main" is located
    let main_pos = line1_str.find("main").unwrap();
    let main_x = 33 + main_pos as u16;
    let m_cell = &buffer[(main_x, 4)];
    assert_eq!(m_cell.fg, Color::Blue);

    // Line 2: "  2       let msg = \"hello world\";"
    let mut line2_str = String::new();
    for x in 33..75 {
        line2_str.push_str(buffer[(x, 5)].symbol());
    }
    assert!(line2_str.contains("let msg = \"hello world\";"));

    let string_pos = line2_str.find("\"hello world\"").unwrap();
    let string_x = 33 + string_pos as u16;
    let quote_cell = &buffer[(string_x, 5)];
    assert_eq!(quote_cell.fg, Color::Green);
}

#[test]
fn test_current_line_git_blame_rendering_and_navigation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    let run = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
    };

    run(&["init"]);
    run(&["config", "user.name", "Alice Tester"]);
    run(&["config", "user.email", "alice@example.com"]);

    std::fs::write(repo_path.join("file.py"), "def line_one():\n    pass\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "add line_one function"]);

    std::fs::write(
        repo_path.join("file.py"),
        "def line_one():\n    pass\n\ndef line_two():\n    pass\n",
    )
    .unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "add line_two function"]);

    let mut app = AppState::new(repo_path.to_path_buf());
    app.files = vec!["file.py".to_string()];
    app.file_selected = 0;
    app.load_currently_selected_file();

    // Line 1 should be selected (cursor_line = 1)
    assert_eq!(app.cursor_line, 1);
    assert!(app.current_line_blame.is_some());
    let blame1 = app.current_line_blame.as_ref().unwrap();
    assert_eq!(blame1.author, "Alice Tester");
    assert_eq!(blame1.summary, "add line_one function");

    let backend = TestBackend::new(140, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg1.contains("Alice Tester"));
    assert!(dbg1.contains("add line_one function"));

    // Move cursor down in CodeViewer -> cursor_line = 4
    app.active_panel = ActivePanel::CodeViewer;
    app.move_selection_down();
    app.move_selection_down();
    app.move_selection_down();
    assert_eq!(app.cursor_line, 4);
    assert!(app.current_line_blame.is_some());
    let blame4 = app.current_line_blame.as_ref().unwrap();
    assert_eq!(blame4.summary, "add line_two function");

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg4 = format!("{:?}", terminal.backend().buffer());
    assert!(dbg4.contains("add line_two function"));
}

#[test]
fn test_code_viewer_viewport_scrolling_on_cursor_navigation() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["test.txt".to_string()];
    app.file_selected = 0;
    app.code_lines = (1..=20).map(|i| format!("line {}", i)).collect();
    app.active_panel = ActivePanel::CodeViewer;

    // 1. Initial State: cursor on line 1
    // Total height = 15. Header (3), Footer (3), Main workspace height = 9.
    // Code Viewer inner content height = 9 - 2 (borders) = 7 lines.
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg1 = format!("{:?}", terminal.backend().buffer());

    // Line 1 should be visible
    assert!(dbg1.contains("1 > line 1"));
    assert!(dbg1.contains("7   line 7"));
    // Line 10 should NOT be visible yet
    assert!(!dbg1.contains("10   line 10"));

    // 2. Move cursor down past bottom of viewport (e.g. to line 11)
    for _ in 0..10 {
        app.move_selection_down();
    }
    assert_eq!(app.cursor_line, 11);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg2 = format!("{:?}", terminal.backend().buffer());

    // Line 11 should now be visible at the cursor, keeping scrolloff context
    assert!(dbg2.contains("11 > line 11"));
    assert!(dbg2.contains(" 8   line 8"));
    // Line 1 should have scrolled off the top
    assert!(!dbg2.contains("  1   line 1"));

    // 3. Move cursor back up (e.g. back to line 3)
    for _ in 0..8 {
        app.move_selection_up();
    }
    assert_eq!(app.cursor_line, 3);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg3 = format!("{:?}", terminal.backend().buffer());

    // Line 3 should now be visible at the top of the scrolled viewport
    assert!(dbg3.contains("3 > line 3"));
}

#[test]
fn test_vim_scrolling_and_positioning_actions() {
    let mut app = AppState::new(PathBuf::from("."));
    app.code_lines = (1..=100).map(|i| format!("line {}", i)).collect();
    app.active_panel = ActivePanel::CodeViewer;
    app.code_viewport_height = 20;
    app.scrolloff = 3;
    app.cursor_line = 1;

    // 1. Half page down (<C-d>)
    app.dispatch_action(Action::HalfPageDown);
    assert_eq!(app.cursor_line, 11);

    // 2. Full page down (<C-f>)
    app.dispatch_action(Action::PageDown);
    assert_eq!(app.cursor_line, 29);

    // 3. Half page up (<C-u>)
    app.dispatch_action(Action::HalfPageUp);
    assert_eq!(app.cursor_line, 19);

    // 4. Center cursor (zz)
    app.cursor_line = 50;
    app.dispatch_action(Action::CenterCursor);
    assert_eq!(app.code_scroll_offset, 39); // (50-1) - 10 = 39

    // 5. Cursor top (zt)
    app.dispatch_action(Action::CursorTop);
    assert_eq!(app.code_scroll_offset, 49); // (50-1) = 49

    // 6. Cursor bottom (zb)
    app.dispatch_action(Action::CursorBottom);
    assert_eq!(app.code_scroll_offset, 30); // (50-1) - 19 = 30
}

#[test]
fn test_sidebar_viewport_scrolling_on_selection() {
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = (1..=15).map(|i| format!("file_{}.rs", i)).collect();
    app.sidebar_view = SidebarView::FileExplorer;
    app.file_selected = 12; // file_13.rs

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    // File 13 should be scrolled into view
    assert!(dbg.contains("file_13.rs"));
    // File 1 should have scrolled out of view
    assert!(!dbg.contains("file_1.rs"));
}

#[test]
fn test_commit_timeline_candidate_highlighting() {
    let backend = TestBackend::new(100, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.commits = vec![
        ("1111111", "Candidate Commit").into(),
        ("2222222", "Non-candidate Commit").into(),
    ];
    app.candidate_commits = vec![("1111111", "Candidate Commit").into()];
    app.sidebar_view = SidebarView::CommitTimeline;

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    // Candidate commit should have '*' indicator prefix
    assert!(dbg.contains("* 1111111 Candidate Commit"));
    // Non-candidate commit should have spaces prefix
    assert!(dbg.contains("  2222222 Non-candidate Commit"));
}

#[test]
fn test_diff_line_highlighting_rendering() {
    use git_tardis::git::DiffLineType;

    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["main.rs".to_string()];
    app.code_lines = vec![
        "fn main() {".to_string(),
        "    println!(\"modified\");".to_string(),
        "    println!(\"added\");".to_string(),
        "}".to_string(),
    ];

    // Explicitly set diff line highlight types for file content lines
    app.file_diff_highlights.insert(2, DiffLineType::Modified);
    app.file_diff_highlights.insert(3, DiffLineType::Added);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    // Line 2 modified indicator "~ " and Line 3 added indicator "+ "
    assert!(dbg.contains("2 ~"));
    assert!(dbg.contains("3 +"));

    // Check raw unified diff rendering
    let mut app_diff = AppState::new(PathBuf::from("."));
    app_diff.code_lines = vec![
        "diff --git a/main.rs b/main.rs".to_string(),
        "@@ -1,3 +1,4 @@".to_string(),
        " fn main() {".to_string(),
        "+    println!(\"added line\");".to_string(),
        "-    println!(\"deleted line\");".to_string(),
    ];

    terminal.draw(|f| render(f, &mut app_diff)).unwrap();
    let dbg_diff = format!("{:?}", terminal.backend().buffer());

    assert!(dbg_diff.contains("diff --git"));
    assert!(dbg_diff.contains("@@ -1,3 +1,4 @@"));
    assert!(dbg_diff.contains("added line"));
    assert!(dbg_diff.contains("deleted line"));
}
