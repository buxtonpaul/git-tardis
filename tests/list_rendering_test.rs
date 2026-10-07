use git_tardis::app::{ActivePanel, AppState, CommitSummary, ModifiedFileEntry, SidebarView};
use git_tardis::ui::render;
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};
use std::path::PathBuf;
use std::time::Instant;

fn rows(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect()
}

fn draw(app: &mut AppState) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| render(f, app)).unwrap();
    terminal
}

fn commit(i: usize) -> CommitSummary {
    CommitSummary::new(
        format!("{:040x}", i),
        format!("c{:06}", i),
        format!("message {}", i),
    )
}

fn timeline_app(commit_count: usize) -> AppState {
    let mut app = AppState::new(PathBuf::from("/nonexistent/git-tardis-test"));
    app.git_version = Some("git version 0.0.0-test".to_string());
    app.commits = (0..commit_count).map(commit).collect();
    app.sidebar_view = SidebarView::CommitTimeline;
    app.active_panel = ActivePanel::Sidebar;
    app
}

#[test]
fn test_row_lookups_match_the_full_lists() {
    let mut app = timeline_app(50);
    app.candidate_commits = vec![commit(3), commit(40)];

    for dirty in [false, true] {
        app.dirty_files = if dirty {
            vec![ModifiedFileEntry::new("changed.rs", "M")]
        } else {
            Vec::new()
        };
        let all = app.display_commits();
        let candidates = app.display_candidate_commits();

        assert_eq!(app.display_commit_count(), all.len());
        assert_eq!(app.display_candidate_count(), candidates.len());
        for i in 0..all.len() + 2 {
            assert_eq!(app.display_commit_at(i), all.get(i).cloned(), "row {}", i);
        }
        for i in 0..candidates.len() + 2 {
            assert_eq!(app.display_candidate_at(i), candidates.get(i).cloned());
        }
        for hash in [
            commit(0).hash,
            commit(40).hash,
            "DIRTY".to_string(),
            "ffff".to_string(),
        ] {
            assert_eq!(
                app.display_commit_position(&hash),
                all.iter().position(|c| c.matches_hash(&hash)),
                "dirty={} hash={}",
                dirty,
                hash
            );
            assert_eq!(
                app.display_candidate_position(&hash),
                candidates.iter().position(|c| c.matches_hash(&hash))
            );
        }
    }
}

#[test]
fn test_timeline_shows_rows_around_a_deep_selection() {
    let mut app = timeline_app(5000);
    app.candidate_commits = vec![commit(2500), commit(2503)];
    app.commit_selected = 2500;

    let terminal = draw(&mut app);
    let screen = rows(&terminal);
    let text = screen.join("\n");

    assert!(
        text.contains("* c002500 message 2500"),
        "selected candidate row"
    );
    assert!(
        text.contains("* c002503 message 2503"),
        "nearby candidate row"
    );
    assert!(text.contains("  c002501 message 2501"), "non-candidate row");
    assert!(text.contains("c002490"), "rows above the selection");
    assert!(
        !text.contains("c000000"),
        "rows far from the selection are not drawn"
    );

    // The selected row is highlighted, and it is the only one.
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    let highlighted: Vec<usize> = (0..screen.len())
        .filter(|&y| buffer.content()[y * width + 2].bg == Color::Blue)
        .collect();
    assert_eq!(highlighted.len(), 1);
    assert!(screen[highlighted[0]].contains("c002500"));
}

#[test]
fn test_timeline_selection_at_either_end() {
    let mut app = timeline_app(5000);

    app.commit_selected = 0;
    let text = rows(&draw(&mut app)).join("\n");
    assert!(text.contains("c000000"));

    app.commit_selected = 4999;
    let text = rows(&draw(&mut app)).join("\n");
    assert!(text.contains("c004999"));
    assert!(!text.contains("c000000"));

    // A stale selection past the end is clamped rather than drawing nothing.
    app.commit_selected = 9999;
    let text = rows(&draw(&mut app)).join("\n");
    assert!(text.contains("c004999"));
}

#[test]
fn test_candidate_timeline_includes_dirty_row() {
    let mut app = timeline_app(10);
    app.timeline_filter = git_tardis::app::TimelineFilter::Candidates;
    app.candidate_commits = (0..300).map(commit).collect();
    app.dirty_files = vec![ModifiedFileEntry::new("changed.rs", "M")];

    app.candidate_selected = 0;
    let text = rows(&draw(&mut app)).join("\n");
    assert!(text.contains("*DIRTY*"));
    assert!(text.contains("* c000000 message 0"));

    app.candidate_selected = 200; // row 200 is commit 199, after the dirty row
    let text = rows(&draw(&mut app)).join("\n");
    assert!(text.contains("* c000199 message 199"));
    assert!(!text.contains("*DIRTY*"));
}

#[test]
fn test_explorer_shows_rows_around_a_deep_selection() {
    let mut app = AppState::new(PathBuf::from("/nonexistent/git-tardis-test"));
    app.git_version = Some("git version 0.0.0-test".to_string());
    app.files = (0..5000).map(|i| format!("file_{:05}.txt", i)).collect();
    app.dirty_files = vec![ModifiedFileEntry::new("file_02501.txt", "M")];
    app.active_panel = ActivePanel::Sidebar;
    app.file_selected = 2500;

    let terminal = draw(&mut app);
    let screen = rows(&terminal);
    let text = screen.join("\n");

    assert!(text.contains("file_02500.txt"));
    assert!(
        text.contains("file_02501.txt [M]"),
        "status badge on a windowed row"
    );
    assert!(!text.contains("file_00000.txt"));

    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    let highlighted: Vec<usize> = (0..screen.len())
        .filter(|&y| buffer.content()[y * width + 2].bg == Color::Blue)
        .collect();
    assert_eq!(highlighted.len(), 1);
    assert!(screen[highlighted[0]].contains("file_02500.txt"));
}

#[test]
fn test_modified_files_shows_rows_around_a_deep_selection() {
    let mut app = AppState::new(PathBuf::from("/nonexistent/git-tardis-test"));
    app.git_version = Some("git version 0.0.0-test".to_string());
    app.dirty_files = (0..2000)
        .map(|i| ModifiedFileEntry::new(format!("changed_{:04}.rs", i), "M"))
        .collect();
    app.sidebar_view = SidebarView::ModifiedFiles;
    app.active_panel = ActivePanel::Sidebar;
    app.dirty_selected = 1999;

    let text = rows(&draw(&mut app)).join("\n");

    assert!(text.contains("changed_1999.rs"));
    assert!(!text.contains("changed_0000.rs"));
}

#[test]
fn test_frame_time_does_not_grow_with_list_length() {
    let mut app = timeline_app(200_000);
    app.candidate_commits = (0..200).map(|i| commit(i * 1000)).collect();
    app.commit_selected = 100_000;
    let mut terminal = Terminal::new(TestBackend::new(200, 60)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();

    let frames = 20;
    let start = Instant::now();
    for i in 0..frames {
        app.commit_selected = 100_000 + i as usize;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
    let per_frame = start.elapsed() / frames;
    println!("200,000-commit timeline: {:?} per frame", per_frame);
    assert!(
        per_frame.as_millis() < 50,
        "frame took {:?}, expected only visible rows to be built",
        per_frame
    );
}
