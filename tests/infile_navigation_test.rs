use std::path::PathBuf;

use git_tardis::app::{ActivePanel, AppState, InputPrompt};
use git_tardis::ui::{render, Action};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_goto_line_navigation() {
    let mut app = AppState::new(PathBuf::from("."));
    app.code_lines = (1..=50).map(|i| format!("fn line_{}() {{}}", i)).collect();
    app.code_viewport_height = 10;

    assert_eq!(app.cursor_line, 1);

    // Jump directly to line 25
    app.goto_line(25);
    assert_eq!(app.cursor_line, 25);
    assert!(app.status_message.contains("Jumped to line 25"));

    // Jump via Action::PromptGotoLine and input buffer submission
    app.dispatch_action(Action::PromptGotoLine);
    assert_eq!(app.input_prompt, Some(InputPrompt::GotoLine));

    app.input_buffer = "42".to_string();
    app.submit_input_prompt();
    assert_eq!(app.cursor_line, 42);
    assert_eq!(app.input_prompt, None);
}

#[test]
fn test_search_text_navigation_and_cycling() {
    let mut app = AppState::new(PathBuf::from("."));
    app.code_lines = vec![
        "fn alpha() {}".to_string(),      // line 1
        "fn beta() {}".to_string(),       // line 2
        "// TODO: fix alpha".to_string(), // line 3
        "fn gamma() {}".to_string(),      // line 4
        "// alpha again".to_string(),     // line 5
    ];

    // Search for "alpha"
    app.search_text("alpha");
    assert_eq!(app.search_matches, vec![1, 3, 5]);
    assert_eq!(app.cursor_line, 1);

    // Next match (line 3)
    app.dispatch_action(Action::SearchNext);
    assert_eq!(app.cursor_line, 3);

    // Next match (line 5)
    app.dispatch_action(Action::SearchNext);
    assert_eq!(app.cursor_line, 5);

    // Next match wraps around to line 1
    app.dispatch_action(Action::SearchNext);
    assert_eq!(app.cursor_line, 1);

    // Previous match wraps back to line 5
    app.dispatch_action(Action::SearchPrev);
    assert_eq!(app.cursor_line, 5);
}

#[test]
fn test_search_symbol_tree_sitter_extraction_and_jump() {
    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["src/lib.rs".to_string()];
    app.active_file = Some("src/lib.rs".to_string());
    app.code_lines = vec![
        "// Comment".to_string(),                 // 1
        "pub struct AppConfig {".to_string(),     // 2
        "    pub name: String,".to_string(),      // 3
        "}".to_string(),                          // 4
        "pub fn initialize_app() {}".to_string(), // 5
    ];

    // Trigger symbol search prompt
    app.dispatch_action(Action::PromptSearchSymbol);
    assert_eq!(app.input_prompt, Some(InputPrompt::SearchSymbol));
    assert!(app.symbol_matches.len() >= 2);

    // Select second symbol (initialize_app at line 5)
    app.input_buffer = "initialize".to_string();
    let filtered = app.filtered_symbols("initialize");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "initialize_app");
    assert_eq!(filtered[0].line_number, 5);

    app.submit_input_prompt();
    assert_eq!(app.cursor_line, 5);
    assert_eq!(app.input_prompt, None);
}

#[test]
fn test_navigation_prompt_modal_rendering() {
    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = AppState::new(PathBuf::from("."));
    app.files = vec!["main.rs".to_string()];
    app.active_file = Some("main.rs".to_string());
    app.code_lines = vec!["fn main() {}".to_string()];
    app.active_panel = ActivePanel::CodeViewer;

    app.dispatch_action(Action::PromptGotoLine);
    app.input_buffer = "100".to_string();

    terminal.draw(|f| render(f, &mut app)).unwrap();
    let dbg = format!("{:?}", terminal.backend().buffer());

    assert!(dbg.contains("Go to Line"));
    assert!(dbg.contains(": 100"));
    assert!(dbg.contains("Press <Enter> to Jump"));
}
