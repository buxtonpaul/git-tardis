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
    assert_eq!(app.visible_file_items()[app.file_selected].name, "app/");

    // Pressing CollapseFolder again on an expanded directory node collapses the directory
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(app.visible_file_items()[app.file_selected].name, "app/");
    assert!(!app.visible_file_items()[app.file_selected].is_expanded);

    // Pressing CollapseFolder on a collapsed directory jumps selection to top parent ("src/")
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(app.visible_file_items()[app.file_selected].name, "src/");

    // Pressing CollapseFolder on an expanded top parent ("src/") collapses it
    app.dispatch_action(Action::CollapseFolder);
    assert_eq!(app.visible_file_items()[app.file_selected].name, "src/");
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

#[test]
fn test_file_explorer_modified_file_highlighting() {
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

    // Set dirty/modified files
    app.dirty_files = vec![
        git_tardis::app::ModifiedFileEntry::new("src/app/mod.rs", "M"),
        git_tardis::app::ModifiedFileEntry::new("README.md", "??"),
    ];

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    // Modified file badges
    assert!(dbg.contains("mod.rs [M]"));
    assert!(dbg.contains("README.md [??]"));

    // Directories containing modified files get [*] indicator badge
    assert!(dbg.contains("src/ [*]"));
    assert!(dbg.contains("app/ [*]"));

    // Clean file has no status badge
    assert!(!dbg.contains("main.rs ["));
}

#[test]
fn test_folder_expansion_state_preserved_across_commit_switches_and_reset() {
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

    // Commit 1: Add src/app/mod.rs and docs/guide.md
    std::fs::create_dir_all(repo_path.join("src/app")).unwrap();
    std::fs::create_dir_all(repo_path.join("docs")).unwrap();
    std::fs::write(repo_path.join("src/app/mod.rs"), "fn main() {}\n").unwrap();
    std::fs::write(repo_path.join("docs/guide.md"), "# Guide\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 1"]);

    // Commit 2: Add src/app/extra.rs
    std::fs::write(repo_path.join("src/app/extra.rs"), "// extra\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 2"]);

    let repo = git_tardis::git::GitRepo::open(repo_path).unwrap();
    let history = repo.get_commit_history(None).unwrap();

    let commit2_hash = history[0].hash.clone(); // latest
    let commit1_hash = history[1].hash.clone(); // earlier

    let mut app = AppState::new(repo_path.to_path_buf());
    app.files = repo.list_files().unwrap();
    app.expand_all_folders();

    // Verify initially "docs" and "src" and "src/app" are all expanded
    assert!(app.expanded_folders.contains("docs"));
    assert!(app.expanded_folders.contains("src"));
    assert!(app.expanded_folders.contains("src/app"));

    // User collapses "src/app"
    app.collapse_folder("src/app");
    assert!(!app.expanded_folders.contains("src/app"));

    // 1. Switch time-travel target to Commit 1
    app.update_state_for_commit_hash(commit1_hash.clone());

    // "src/app" MUST remain collapsed
    assert!(!app.expanded_folders.contains("src/app"));
    assert!(app.expanded_folders.contains("docs"));
    assert!(app.expanded_folders.contains("src"));

    // 2. Switch time-travel target to Commit 2
    app.update_state_for_commit_hash(commit2_hash.clone());

    // "src/app" MUST still remain collapsed
    assert!(!app.expanded_folders.contains("src/app"));

    // 3. Reset time travel (back to working directory)
    app.reset_time_travel();

    // "src/app" MUST still remain collapsed
    assert!(!app.expanded_folders.contains("src/app"));
}
