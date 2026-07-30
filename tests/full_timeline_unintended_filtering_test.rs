use std::fs;
use std::process::Command;
use tempfile::TempDir;

use git_tardis::app::{AppState, NavigationMode, SidebarView, TimelineFilter};
use git_tardis::ui::Action;

fn setup_large_repo() -> (TempDir, std::path::PathBuf) {
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
    run(&["config", "user.name", "Issue67 Tester"]);
    run(&["config", "user.email", "issue67@example.com"]);

    // Create 60 commits total:
    // 5 commits modify fileA.txt, 55 commits modify fileB.txt
    for i in 1..=60 {
        if i % 12 == 0 {
            let content = format!("fileA content iteration {}\n", i);
            fs::write(repo_path.join("fileA.txt"), content).unwrap();
            run(&["add", "fileA.txt"]);
        } else {
            let content = format!("fileB content iteration {}\n", i);
            fs::write(repo_path.join("fileB.txt"), content).unwrap();
            run(&["add", "fileB.txt"]);
        }
        let msg = format!("Commit #{}", i);
        run(&["commit", "-m", &msg]);
    }

    (temp_dir, repo_path)
}

#[test]
fn test_full_commit_timeline_displays_all_commits_without_candidate_filtering() {
    let (_dir, repo_path) = setup_large_repo();

    let mut app = AppState::new(repo_path);
    app.reload_repo_data();

    // 1. Initial full commit history check (60 commits)
    assert_eq!(app.commits.len(), 60);
    assert_eq!(app.display_commits().len(), 60);

    // 2. Select fileA.txt (which only has 5 candidate commits)
    if let Some(idx) = app.visible_file_items().iter().position(|item| item.path == "fileA.txt") {
        app.file_selected = idx;
        app.load_currently_selected_file();
    } else {
        app.active_file = Some("fileA.txt".to_string());
        app.load_currently_selected_file();
    }

    // Set NavigationMode to File
    app.set_navigation_mode(NavigationMode::File);
    assert_eq!(app.candidate_commits.len(), 5);

    // 3. Set sidebar view to CommitTimeline (Option 3)
    app.dispatch_action(Action::SetSidebarView(3));
    assert_eq!(app.sidebar_view, SidebarView::CommitTimeline);
    assert_eq!(app.timeline_filter, TimelineFilter::All);

    // Full commit timeline must STILL show all 60 commits, not 5 candidate commits
    let full_commits = app.display_commits();
    assert_eq!(full_commits.len(), 60);

    // 4. Switch to Candidate Timeline (Option 4)
    app.dispatch_action(Action::SetSidebarView(4));
    assert_eq!(app.timeline_filter, TimelineFilter::Candidates);
    let candidate_commits = app.display_candidate_commits();
    assert_eq!(candidate_commits.len(), 5);

    // 5. Switching back to Commit Timeline (Option 3) resets filter to ALL
    app.dispatch_action(Action::SetSidebarView(3));
    assert_eq!(app.timeline_filter, TimelineFilter::All);
    assert_eq!(app.display_commits().len(), 60);
}
