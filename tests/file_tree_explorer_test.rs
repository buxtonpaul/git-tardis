use std::path::PathBuf;

use git_tardis::app::{ActivePanel, AppState, SidebarView};
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_file_tree_folding_and_unfolding_actions() {
    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec![
        "README.md".to_string(),
        "src/app/mod.rs".to_string(),
        "src/app/types.rs".to_string(),
        "src/main.rs".to_string(),
    ];
    app.expand_all_folders();

    // With all folders expanded initially:
    // 0: ▼ src/
    // 1:   ▼ app/
    // 2:     mod.rs
    // 3:     types.rs
    // 4:   main.rs
    // 5: README.md
    let visible = app.visible_file_items();
    assert_eq!(visible.len(), 6);
    assert_eq!(visible[0].name, "src/");
    assert_eq!(visible[1].name, "app/");
    assert_eq!(visible[2].name, "mod.rs");

    // Select index 1 ("src/app") and collapse it
    app.file_selected = 1;
    app.dispatch_action(Action::CollapseFolder);

    // After collapsing "src/app":
    // 0: ▼ src/
    // 1:   ▶ app/
    // 2:   main.rs
    // 3: README.md
    let visible_after_collapse = app.visible_file_items();
    assert_eq!(visible_after_collapse.len(), 4);
    assert_eq!(visible_after_collapse[1].name, "app/");
    assert!(!visible_after_collapse[1].is_expanded);
    assert_eq!(visible_after_collapse[2].name, "main.rs");

    // Expand "src/app" again via Action::ExpandFolder
    app.file_selected = 1;
    app.dispatch_action(Action::ExpandFolder);

    let visible_after_expand = app.visible_file_items();
    assert_eq!(visible_after_expand.len(), 6);
    assert_eq!(visible_after_expand[1].name, "app/");
    assert!(visible_after_expand[1].is_expanded);
    assert_eq!(visible_after_expand[2].name, "mod.rs");
}

#[test]
fn test_file_tree_keyboard_navigation_and_parent_jumping() {
    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec![
        "README.md".to_string(),
        "src/app/mod.rs".to_string(),
        "src/app/types.rs".to_string(),
        "src/main.rs".to_string(),
    ];
    app.expand_all_folders();
    app.sidebar_view = SidebarView::FileExplorer;

    // Index 2 is "src/app/mod.rs" (file node at depth 2)
    app.file_selected = 2;
    assert_eq!(app.visible_file_items()[app.file_selected].name, "mod.rs");

    // Pressing CollapseFolder ('h' or '<Left>') on a file node jumps selection to parent directory ("src/app")
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(
        app.visible_file_items()[app.file_selected].name,
        "app/"
    );

    // Pressing CollapseFolder again on an expanded directory node collapses the directory
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(
        app.visible_file_items()[app.file_selected].name,
        "app/"
    );
    assert!(!app.visible_file_items()[app.file_selected].is_expanded);

    // Pressing CollapseFolder on a collapsed directory jumps selection to top parent ("src/")
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(
        app.visible_file_items()[app.file_selected].name,
        "src/"
    );

    // Pressing CollapseFolder on an expanded top parent ("src/") collapses it
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(
        app.visible_file_items()[app.file_selected].name,
        "src/"
    );
    assert!(!app.visible_file_items()[app.file_selected].is_expanded);

    // Pressing ExpandFolder ('<Right>') on a collapsed directory expands it
    app.dispatch_action(Action::ExpandFolder);
    assert!(app.visible_file_items()[app.file_selected].is_expanded);
}

#[test]
fn test_file_explorer_rendering_tree_nodes() {
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec![
        "README.md".to_string(),
        "src/app/mod.rs".to_string(),
        "src/main.rs".to_string(),
    ];
    app.expand_all_folders();
    app.sidebar_view = SidebarView::FileExplorer;
    app.active_panel = ActivePanel::Sidebar;

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    // Verify tree indentation and fold indicator rendering
    assert!(dbg.contains("▼ src/"));
    assert!(dbg.contains("▼ app/"));
    assert!(dbg.contains("mod.rs"));
    assert!(dbg.contains("main.rs"));
    assert!(dbg.contains("README.md"));
}
