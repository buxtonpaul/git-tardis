use git_tardis::app::{ActivePanel, AppState, NavigationMode, SidebarView};
use std::fs;
use tempfile::TempDir;

fn create_large_dummy_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path();

    let output = std::process::Command::new("git")
        .arg("init")
        .current_dir(repo_path)
        .output()
        .unwrap();
    assert!(output.status.success());

    // Create a deeply nested directory structure with dummy files
    for d in 0..5 {
        for f in 0..100 {
            let sub_dir = repo_path.join(format!("src/module_{}/sub_{}", d, f / 10));
            fs::create_dir_all(&sub_dir).unwrap();
            let file_path = sub_dir.join(format!("file_{}.rs", f));
            fs::write(
                &file_path,
                format!(
                    "fn fn_{}() {{\n    let x = {};\n    println!(\"hello {}\");\n}}\n",
                    f, f, f
                ),
            )
            .unwrap();
        }
    }

    // Git add and commit
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit with 500 files"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    dir
}

#[test]
fn test_large_file_tree_caching_and_fast_lookup() {
    let temp = create_large_dummy_repo();
    let mut app = AppState::new(temp.path().to_path_buf());
    app.reload_repo_data();

    assert!(app.files.len() >= 500);

    // Initial call to visible_file_items should build the tree
    let start = std::time::Instant::now();
    let visible1 = app.visible_file_items();
    let duration_initial = start.elapsed();

    // Subsequent calls should hit cache and be near instant
    let start_cached = std::time::Instant::now();
    for _ in 0..1000 {
        let visible = app.visible_file_items();
        assert_eq!(visible.len(), visible1.len());
    }
    let duration_cached = start_cached.elapsed();

    println!("Initial file tree build: {:?}", duration_initial);
    println!(
        "1000 cached visible_file_items calls: {:?}",
        duration_cached
    );

    assert!(
        duration_cached.as_millis() < 50,
        "1000 cached calls took too long: {:?}",
        duration_cached
    );
}

#[test]
fn test_line_navigation_candidate_commit_query_deduplication() {
    let temp = create_large_dummy_repo();
    let mut app = AppState::new(temp.path().to_path_buf());
    app.reload_repo_data();

    app.active_panel = ActivePanel::CodeViewer;
    app.nav_mode = NavigationMode::File;

    let target_file = app
        .files
        .iter()
        .find(|f| f.ends_with("file_0.rs"))
        .unwrap()
        .clone();
    app.active_file = Some(target_file);
    app.load_currently_selected_file();

    // Move cursor down 50 times in File navigation mode
    let start = std::time::Instant::now();
    for _ in 0..50 {
        app.move_selection_down();
    }
    let duration = start.elapsed();

    println!("50 line movements in CodeViewer: {:?}", duration);
    assert!(
        duration.as_millis() < 100,
        "Line navigation was too slow: {:?}",
        duration
    );
}

#[test]
fn test_file_explorer_unmodified_diff_skipping() {
    let temp = create_large_dummy_repo();
    let mut app = AppState::new(temp.path().to_path_buf());
    app.reload_repo_data();
    app.expand_all_folders();

    app.active_panel = ActivePanel::Sidebar;
    app.sidebar_view = SidebarView::FileExplorer;

    // Moving through 20 items in file tree
    let start = std::time::Instant::now();
    for _ in 0..20 {
        app.move_selection_down();
    }
    let duration = start.elapsed();

    // Unmodified files skip diff generation
    assert!(app.file_diff_highlights.is_empty());

    println!("20 file selections in FileExplorer: {:?}", duration);
    assert!(
        duration.as_millis() < 1000,
        "FileExplorer selection was too slow: {:?}",
        duration
    );
}
