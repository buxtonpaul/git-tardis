use git_tardis::app::AppState;

#[test]
fn test_line_position_tracking_and_viewport_alignment_across_commits() {
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

    let file_c1 = vec![
        "// line 1",
        "// line 2",
        "// line 3",
        "// line 4",
        "// TARGET CODE LINE",
        "// line 6",
        "// line 7",
        "// line 8",
        "// line 9",
        "// line 10",
        "// line 11",
        "// line 12",
        "// line 13",
        "// line 14",
        "// line 15",
    ]
    .join("\n")
        + "\n";

    std::fs::create_dir_all(repo_path.join("src")).unwrap();
    std::fs::write(repo_path.join("src/main.rs"), &file_c1).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 1"]);

    // Commit 2: Add 5 lines before TARGET CODE LINE
    let file_c2 = vec![
        "// line 1",
        "// line 2",
        "// line 3",
        "// line 4",
        "// added line A",
        "// added line B",
        "// added line C",
        "// added line D",
        "// added line E",
        "// TARGET CODE LINE",
        "// line 6",
        "// line 7",
        "// line 8",
        "// line 9",
        "// line 10",
        "// line 11",
        "// line 12",
        "// line 13",
        "// line 14",
        "// line 15",
    ]
    .join("\n")
        + "\n";

    std::fs::write(repo_path.join("src/main.rs"), &file_c2).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 2"]);

    // Commit 3: Delete 3 lines before TARGET CODE LINE
    let file_c3 = vec![
        "// line 1",
        "// line 2",
        "// added line A",
        "// added line B",
        "// added line C",
        "// added line D",
        "// added line E",
        "// TARGET CODE LINE",
        "// line 6",
        "// line 7",
        "// line 8",
        "// line 9",
        "// line 10",
        "// line 11",
        "// line 12",
        "// line 13",
        "// line 14",
        "// line 15",
    ]
    .join("\n")
        + "\n";

    std::fs::write(repo_path.join("src/main.rs"), &file_c3).unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "Commit 3"]);

    let repo = git_tardis::git::GitRepo::open(repo_path).unwrap();
    let history = repo.get_commit_history(None).unwrap();

    let commit3_hash = history[0].hash.clone(); // latest
    let commit2_hash = history[1].hash.clone();
    let commit1_hash = history[2].hash.clone(); // oldest

    let mut app = AppState::new(repo_path.to_path_buf());
    app.files = repo.list_files().unwrap();
    app.expand_all_folders();
    app.sidebar_view = git_tardis::app::SidebarView::FileExplorer;
    // Find index of "src/main.rs" in visible file items
    let visible = app.visible_file_items();
    let idx = visible
        .iter()
        .position(|it| it.path == "src/main.rs")
        .unwrap();
    app.file_selected = idx;
    app.code_viewport_height = 10;

    // 1. Initial state at working directory (matches Commit 3)
    app.load_currently_selected_file();
    assert_eq!(app.code_lines[7], "// TARGET CODE LINE"); // line 8 (1-indexed)

    // Position cursor on TARGET CODE LINE (line 8)
    app.cursor_line = 8;
    app.code_scroll_offset = 3; // visual row: (8 - 3 - 1) = 4

    // 2. Transition time-travel to Commit 1 (earliest)
    app.update_state_for_commit_hash(commit1_hash.clone());
    app.load_currently_selected_file();

    // In Commit 1, TARGET CODE LINE was at line 5
    assert_eq!(app.cursor_line, 5);
    assert_eq!(app.code_lines[app.cursor_line - 1], "// TARGET CODE LINE");
    // Visual row offset is preserved: (5 - 0 - 1) = 4
    assert_eq!(app.code_scroll_offset, 0);

    // 3. Transition time-travel to Commit 2
    app.update_state_for_commit_hash(commit2_hash.clone());
    app.load_currently_selected_file();

    // In Commit 2, 5 lines were added before TARGET CODE LINE, placing it at line 10
    assert_eq!(app.cursor_line, 10);
    assert_eq!(app.code_lines[app.cursor_line - 1], "// TARGET CODE LINE");
    // Scroll offset adjusts so visual row is preserved: (10 - 5 - 1) = 4
    assert_eq!(app.code_scroll_offset, 5);

    // 4. Transition time-travel to Commit 3
    app.update_state_for_commit_hash(commit3_hash.clone());
    app.load_currently_selected_file();

    // In Commit 3, TARGET CODE LINE is at line 8
    assert_eq!(app.cursor_line, 8);
    assert_eq!(app.code_lines[app.cursor_line - 1], "// TARGET CODE LINE");
    assert_eq!(app.code_scroll_offset, 3);

    // 5. Exit time-travel (reset to working directory)
    app.reset_time_travel();

    assert_eq!(app.cursor_line, 8);
    assert_eq!(app.code_lines[app.cursor_line - 1], "// TARGET CODE LINE");
    assert_eq!(app.code_scroll_offset, 3);
}
