use git_tardis::app::{ActivePanel, AppState};
use git_tardis::ui::render;
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};
use std::path::PathBuf;
use std::time::Instant;

fn app_with_file(path: &str, lines: Vec<String>) -> AppState {
    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec![path.to_string()];
    app.active_file = Some(path.to_string());
    app.active_panel = ActivePanel::CodeViewer;
    app.code_lines = lines;
    app
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_highlighting_is_parsed_once_across_frames_and_cursor_moves() {
    let lines: Vec<String> = (0..500).map(|i| format!("fn func_{}() {{}}", i)).collect();
    let mut app = app_with_file("src/big.rs", lines);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();

    for _ in 0..5 {
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
    assert_eq!(app.highlight_parse_count, 1);

    for _ in 0..100 {
        app.move_selection_down();
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
    assert_eq!(app.highlight_parse_count, 1);
    assert!(screen_text(&terminal).contains("fn func_100() {}"));
}

#[test]
fn test_highlighting_is_refreshed_when_content_changes() {
    let mut app = app_with_file("src/a.rs", vec!["fn before() {}".to_string()]);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();

    terminal.draw(|f| render(f, &mut app)).unwrap();
    assert!(screen_text(&terminal).contains("fn before() {}"));

    app.code_lines = vec!["fn after() {}".to_string()];
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("fn after() {}"));
    assert!(!text.contains("fn before() {}"));
    assert_eq!(app.highlight_parse_count, 2);
}

#[test]
fn test_no_stale_highlighting_after_switching_to_file_without_grammar() {
    let mut app = app_with_file(
        "src/a.rs",
        vec!["fn main() {}".to_string(), "}".to_string()],
    );
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();

    app.active_file = Some("notes.txt".to_string());
    app.code_lines = vec!["plain text".to_string(), "second line".to_string()];
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("plain text"));
    assert!(text.contains("second line"));
    assert!(!text.contains("fn main"));
}

#[test]
fn test_keyword_colour_is_preserved_when_scrolled() {
    let mut lines: Vec<String> = (0..300).map(|i| format!("// comment {}", i)).collect();
    lines.push("fn target() {}".to_string());
    let mut app = app_with_file("src/a.rs", lines);
    app.cursor_line = 300; // line above the target, so the target is not cursor-styled
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();

    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    let text = screen_text(&terminal);
    let row = text
        .lines()
        .position(|l| l.contains("fn target() {}"))
        .expect("target line should be visible");
    let row_cells = &buffer.content()[row * width..(row + 1) * width];
    let row_symbols: Vec<&str> = row_cells.iter().map(|c| c.symbol()).collect();
    let col = (0..width - 1)
        .find(|&i| row_symbols[i] == "f" && row_symbols[i + 1] == "n")
        .unwrap();
    assert_eq!(row_cells[col].fg, Color::Magenta);
}

#[test]
fn test_scrolling_past_u16_line_count_shows_last_line() {
    let total = 70_000;
    let lines: Vec<String> = (1..=total).map(|i| format!("row {}", i)).collect();
    let mut app = app_with_file("data.txt", lines);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();

    app.goto_line(total);
    terminal.draw(|f| render(f, &mut app)).unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("row 70000"), "last line should be visible");
    assert!(
        text.contains("70000 > "),
        "cursor gutter should be on the last line"
    );
    assert!(!text.contains("row 4465 "), "must not wrap around at 65536");
}

#[test]
fn test_markdown_code_block_state_survives_scrolling_past_the_fence() {
    let mut lines = vec!["```".to_string()];
    lines.extend((0..200).map(|i| format!("code line {}", i)));
    lines.push("```".to_string());
    let mut app = app_with_file("notes.md", lines);
    app.render_markdown_formatted = true;
    app.cursor_line = 150;
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap();

    assert!(
        app.code_scroll_offset > 1,
        "opening fence should be scrolled out of view"
    );
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    let text = screen_text(&terminal);
    let row = text
        .lines()
        .position(|l| l.contains("code line 140"))
        .expect("code line should be visible");
    let line = text.lines().nth(row).unwrap();
    let byte_col = line.find("code line 140").unwrap();
    let col = line[..byte_col].chars().count();
    assert_eq!(buffer.content()[row * width + col].fg, Color::Green);
}

#[test]
fn test_frame_time_does_not_grow_with_file_length() {
    let lines: Vec<String> = (0..20_000)
        .map(|i| format!("fn func_{}(a: u32) -> u32 {{ a + {} }}", i, i))
        .collect();
    let mut app = app_with_file("src/huge.rs", lines);
    let mut terminal = Terminal::new(TestBackend::new(200, 60)).unwrap();
    terminal.draw(|f| render(f, &mut app)).unwrap(); // first frame parses the file

    // Move the cursor directly so only rendering is timed, not the blame lookup.
    let frames = 20;
    let start = Instant::now();
    for i in 0..frames {
        app.cursor_line = 10_000 + i as usize;
        terminal.draw(|f| render(f, &mut app)).unwrap();
    }
    let per_frame = start.elapsed() / frames;
    println!(
        "20,000-line file: {:?} per frame after the first",
        per_frame
    );
    assert_eq!(app.highlight_parse_count, 1);
    assert!(
        per_frame.as_millis() < 50,
        "frame took {:?}, expected viewport-only rendering",
        per_frame
    );
}
