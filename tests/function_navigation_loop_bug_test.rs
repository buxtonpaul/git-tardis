use std::fs;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{ActivePanel, AppState, NavigationMode, SidebarView};
use git_tardis::ui::Action;

fn setup_shifting_function_repo() -> (TempDir, std::path::PathBuf, Vec<String>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_path_buf();

    let run = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(args)
            .output()
            .expect("Failed to execute git command");
        assert!(
            output.status.success(),
            "Git command failed: git {}\nStderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    };

    run(&["init"]);
    run(&["config", "user.name", "FuncLoop Tester"]);
    run(&["config", "user.email", "funcloop@example.com"]);

    let mut commit_hashes = Vec::new();

    let get_head_hash = || {
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(&["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    // Commit 1: Initial file with target_func near top (line 2)
    let c1 = "fn initial_helper() {}\n\nfn target_func() {\n    let x = 1;\n    println!(\"v1: {}\", x);\n}\n";
    fs::write(repo_path.join("app.rs"), c1).unwrap();
    run(&["add", "app.rs"]);
    run(&["commit", "-m", "C1: Initial target_func"]);
    commit_hashes.push(get_head_hash());

    // Commit 2: Update target_func
    let c2 = "fn initial_helper() {}\n\nfn target_func() {\n    let x = 2;\n    println!(\"v2: {}\", x);\n}\n";
    fs::write(repo_path.join("app.rs"), c2).unwrap();
    run(&["add", "app.rs"]);
    run(&["commit", "-m", "C2: Update target_func v2"]);
    commit_hashes.push(get_head_hash());

    // Commit 3: Insert 50 lines at top of file (shifting target_func down to line 55)
    let mut top_lines = String::new();
    for i in 1..=50 {
        top_lines.push_str(&format!("fn top_filler_{}() {{}}\n", i));
    }
    let c3 = format!(
        "{}\nfn initial_helper() {{}}\n\nfn target_func() {{\n    let x = 2;\n    println!(\"v2: {{}}\", x);\n}}\n",
        top_lines
    );
    fs::write(repo_path.join("app.rs"), c3).unwrap();
    run(&["add", "app.rs"]);
    run(&["commit", "-m", "C3: Insert 50 top filler lines"]);
    commit_hashes.push(get_head_hash());

    // Commit 4: Update target_func at new position
    let c4 = format!(
        "{}\nfn initial_helper() {{}}\n\nfn target_func() {{\n    let x = 3;\n    println!(\"v3: {{}}\", x);\n}}\n",
        top_lines
    );
    fs::write(repo_path.join("app.rs"), c4).unwrap();
    run(&["add", "app.rs"]);
    run(&["commit", "-m", "C4: Update target_func v3"]);
    commit_hashes.push(get_head_hash());

    // Commit 5: Update target_func v4
    let c5 = format!(
        "{}\nfn initial_helper() {{}}\n\nfn target_func() {{\n    let x = 4;\n    println!(\"v4: {{}}\", x);\n}}\n",
        top_lines
    );
    fs::write(repo_path.join("app.rs"), c5).unwrap();
    run(&["add", "app.rs"]);
    run(&["commit", "-m", "C5: Update target_func v4"]);
    commit_hashes.push(get_head_hash());

    (temp_dir, repo_path, commit_hashes)
}

#[test]
fn test_function_timeline_navigation_does_not_loop_or_jump_back() {
    let (_dir, repo_path, commits) = setup_shifting_function_repo();

    let mut app = AppState::new(repo_path);
    app.reload_repo_data();

    // Select app.rs
    app.active_file = Some("app.rs".to_string());
    app.sidebar_view = SidebarView::FileExplorer;
    app.load_currently_selected_file();

    // Position cursor on target_func (line 54 in app.rs at HEAD)
    let target_line = app
        .code_lines
        .iter()
        .position(|l| l.contains("fn target_func"))
        .map(|i| i + 1)
        .expect("target_func should exist at HEAD");
    app.cursor_line = target_line;
    app.active_panel = ActivePanel::CodeViewer;

    // Set NavigationMode to Function
    app.set_navigation_mode(NavigationMode::Function);

    // Initial jump PREV from HEAD -> Should land on C4 (commits[3])
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[3]);

    // Second jump PREV -> Should land on C2 (commits[1])
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[1]);

    // Third jump PREV -> Should land on C1 (commits[0])
    app.dispatch_action(Action::JumpPrevAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[0]);

    // Reverse direction (Jump NEXT)
    // Step forward: C1 -> C2 (commits[1])
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[1]);

    // Step forward: C2 -> C4 (commits[3])
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[3]);

    // Step forward: C4 -> C5 (commits[4])
    app.dispatch_action(Action::JumpNextAuto);
    assert_eq!(app.selected_commit_hash.as_ref().unwrap(), &commits[4]);

    // Step forward: C5 -> HEAD / working directory (None)
    app.dispatch_action(Action::JumpNextAuto);
    assert!(app.selected_commit_hash.is_none());
}
