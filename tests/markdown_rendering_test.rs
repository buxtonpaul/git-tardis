use std::path::PathBuf;

use git_tardis::app::AppState;
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_markdown_formatted_rendering_and_toggle_action() {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["README.md".to_string(), "src/main.rs".to_string()];
    app.active_file = Some("README.md".to_string());
    app.code_lines = vec![
        "# Project Overview".to_string(),
        "Welcome to **Git-tardis**!".to_string(),
        "- [x] Feature complete".to_string(),
        "- Bullet list item".to_string(),
        "```rust".to_string(),
        "fn main() {}".to_string(),
        "```".to_string(),
    ];

    // 1. Initial state: formatted rendering enabled
    assert!(app.render_markdown_formatted);

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg_formatted = format!("{:?}", terminal.backend().buffer());

    // Check that formatted structures are rendered (e.g. header prefix "# ", list marker "• ", check box "[✓] ")
    assert!(dbg_formatted.contains("# "));
    assert!(dbg_formatted.contains("Project Overview"));
    assert!(dbg_formatted.contains("[✓]"));
    assert!(dbg_formatted.contains("• "));
    assert!(dbg_formatted.contains("```rust"));

    // 2. Dispatch Action::ToggleMarkdownFormat (triggered by 'M')
    app.dispatch_action(Action::ToggleMarkdownFormat);
    assert!(!app.render_markdown_formatted);
    assert!(app
        .status_message
        .contains("Markdown formatted rendering: DISABLED"));

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg_raw = format!("{:?}", terminal.backend().buffer());

    // Check that raw Markdown lines are rendered (e.g. "- [x] Feature complete", "- Bullet list item")
    assert!(dbg_raw.contains("# Project Overview"));
    assert!(dbg_raw.contains("- [x] Feature complete"));
    assert!(dbg_raw.contains("- Bullet list item"));

    // 3. Dispatch Action::ToggleMarkdownFormat again to re-enable formatted view
    app.dispatch_action(Action::ToggleMarkdownFormat);
    assert!(app.render_markdown_formatted);
    assert!(app
        .status_message
        .contains("Markdown formatted rendering: ENABLED"));

    // 4. Navigating away from Markdown file to a non-markdown file resets render_markdown_formatted to false
    app.last_loaded_file = Some("README.md".to_string());
    app.open_file_at_line("src/main.rs", None);
    assert!(!app.render_markdown_formatted);

    // 5. Navigating to another Markdown file defaults to formatted rendering enabled (render_markdown_formatted = true)
    app.files.push("docs/guide.md".to_string());
    app.invalidate_file_tree_cache();
    app.open_file_at_line("docs/guide.md", None);
    assert!(app.render_markdown_formatted);
}
