use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::app::{ActivePanel, AppState, SidebarView};
use crate::treesitter::{capture_name_to_style, highlight_viewport};

pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Sidebar tab selector
            Constraint::Min(0),    // Main split workspace
            Constraint::Length(3), // Footer status bar
        ])
        .split(frame.area());

    // 1. Top Header & Tab Selector
    let header_block = Block::default()
        .borders(Borders::ALL)
        .title(" Git-tardis TUI ");

    let sidebar_titles = vec![
        SidebarView::FileExplorer.name(),
        SidebarView::ModifiedFiles.name(),
        SidebarView::CommitTimeline.name(),
    ];

    let selected_tab = match state.sidebar_view {
        SidebarView::FileExplorer => 0,
        SidebarView::ModifiedFiles => 1,
        SidebarView::CommitTimeline => 2,
    };

    let tabs = Tabs::new(sidebar_titles)
        .block(header_block)
        .select(selected_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    // 2. Middle Main Workspace (32% Sidebar / 68% Code Viewer Split)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(32), Constraint::Percentage(68)])
        .split(chunks[1]);

    // Left Sidebar rendering
    let is_sidebar_active = state.active_panel == ActivePanel::Sidebar;
    let sidebar_border_color = if is_sidebar_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Sidebar [{}] ", state.sidebar_view.name()))
        .border_style(Style::default().fg(sidebar_border_color));

    match state.sidebar_view {
        SidebarView::FileExplorer => {
            let items: Vec<ListItem> = state
                .files
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let style = if i == state.file_selected && is_sidebar_active {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else if i == state.file_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(f.as_str()).style(style)
                })
                .collect();
            let list = List::new(items).block(sidebar_block);
            frame.render_widget(list, main_chunks[0]);
        }
        SidebarView::ModifiedFiles => {
            let items: Vec<ListItem> = state
                .modified_files
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let style = if i == state.modified_selected && is_sidebar_active {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else if i == state.modified_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(f.as_str()).style(style)
                })
                .collect();
            let list = List::new(items).block(sidebar_block);
            frame.render_widget(list, main_chunks[0]);
        }
        SidebarView::CommitTimeline => {
            let items: Vec<ListItem> = state
                .commits
                .iter()
                .enumerate()
                .map(|(i, (hash, msg))| {
                    let text = format!("{} {}", hash, msg);
                    let style = if i == state.commit_selected && is_sidebar_active {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else if i == state.commit_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(text).style(style)
                })
                .collect();
            let list = List::new(items).block(sidebar_block);
            frame.render_widget(list, main_chunks[0]);
        }
    }

    // Right Code Viewer rendering
    let is_code_active = state.active_panel == ActivePanel::CodeViewer;
    let code_border_color = if is_code_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title_text = if let Some(blame) = &state.current_line_blame {
        let blame_str = crate::git::format_blame_annotation(blame);
        format!(
            " Code Viewer - Mode: [{}] | Blame: {} ",
            state.nav_mode.name(),
            blame_str
        )
    } else {
        format!(" Code Viewer - Mode: [{}] ", state.nav_mode.name())
    };

    let code_block = Block::default()
        .borders(Borders::ALL)
        .title(title_text)
        .border_style(Style::default().fg(code_border_color));

    let mut formatted_code = Vec::new();
    if state.code_lines.is_empty() {
        formatted_code.push(Line::from(Span::styled(
            "  (No file or code content loaded)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let grammar_entry = state.current_file_path().and_then(|path| {
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            state.grammar_registry.get_by_extension(ext)
        });

        let source_code = state.code_lines.join("\n");
        let highlighted_lines = grammar_entry
            .as_ref()
            .map(|entry| highlight_viewport(entry, &source_code, 1, state.code_lines.len()));

        for (idx, line) in state.code_lines.iter().enumerate() {
            let line_num = idx + 1;
            let is_cursor = line_num == state.cursor_line;
            let prefix = if is_cursor { "> " } else { "  " };

            let gutter_style = if is_cursor && is_code_active {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_cursor {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let gutter_text = format!("{:>3} {}", line_num, prefix);
            let mut spans = vec![Span::styled(gutter_text, gutter_style)];

            if let Some(hl_line) = highlighted_lines.as_ref().and_then(|hl| hl.get(idx)) {
                for hl_span in &hl_line.spans {
                    let mut span_style = capture_name_to_style(&hl_span.capture_name);
                    if is_cursor && is_code_active {
                        span_style = span_style.bg(Color::DarkGray);
                        if span_style.fg.is_none() {
                            span_style = span_style.fg(Color::Yellow);
                        }
                        span_style = span_style.add_modifier(Modifier::BOLD);
                    } else if is_cursor {
                        if span_style.fg.is_none() {
                            span_style = span_style.fg(Color::Yellow);
                        }
                    }
                    spans.push(Span::styled(hl_span.text.clone(), span_style));
                }
            } else {
                let line_style = if is_cursor && is_code_active {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if is_cursor {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(line.clone(), line_style));
            }

            if is_cursor {
                if let Some(blame) = &state.current_line_blame {
                    let annotation = format!("   {}", crate::git::format_blame_annotation(blame));
                    spans.push(Span::styled(
                        annotation,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    ));
                }
            }

            formatted_code.push(Line::from(spans));
        }
    }

    let viewport_height = main_chunks[1].height.saturating_sub(2) as usize;
    let scroll_offset = if state.code_lines.is_empty() || viewport_height == 0 {
        0
    } else {
        let cursor_idx = state.cursor_line.saturating_sub(1);
        if cursor_idx < state.code_scroll_offset {
            cursor_idx
        } else if cursor_idx >= state.code_scroll_offset + viewport_height {
            cursor_idx + 1 - viewport_height
        } else {
            state.code_scroll_offset
        }
    };

    let paragraph = Paragraph::new(formatted_code)
        .block(code_block)
        .scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, main_chunks[1]);

    // 3. Footer Status Bar
    let status_text = format!(
        " Status: {} | Keys: [Tab] Switch Panel | [1/2/3] Sidebar View | [m] Nav Mode | [q] Quit",
        state.status_message
    );
    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Green));
    frame.render_widget(status_bar, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    #[test]
    fn test_ui_rendering_structure() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.files = vec!["src/main.rs".into(), "Cargo.toml".into()];
        app.code_lines = vec!["fn main() {}".into()];

        terminal.draw(|f| render(f, &app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content = format!("{:?}", buffer);

        // Check essential UI titles and sections
        assert!(content.contains("Git-tardis TUI"));
        assert!(content.contains("1: Explorer"));
        assert!(content.contains("2: Modified Files"));
        assert!(content.contains("3: Commit Timeline"));
        assert!(content.contains("Sidebar [1: Explorer]"));
        assert!(content.contains("Code Viewer - Mode: [FILE Mode]"));
        assert!(content.contains("src/main.rs"));
        assert!(content.contains("fn main() {}"));
    }

    #[test]
    fn test_border_color_focus_toggle() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.files = vec!["src/main.rs".into()];

        // Render with Sidebar active
        terminal.draw(|f| render(f, &app)).unwrap();
        let sidebar_rect = main_chunks_0_rect(); // top left panel
        let code_rect = main_chunks_1_rect();

        // Cell at top-left of sidebar block should be Cyan when active
        let sidebar_border_cell = &terminal.backend().buffer()[(sidebar_rect.0, sidebar_rect.1)];
        assert_eq!(sidebar_border_cell.fg, Color::Cyan);

        let code_border_cell = &terminal.backend().buffer()[(code_rect.0, code_rect.1)];
        assert_eq!(code_border_cell.fg, Color::DarkGray);

        // Switch focus to CodeViewer
        app.toggle_panel_focus();
        terminal.draw(|f| render(f, &app)).unwrap();

        let sidebar_border_cell_2 = &terminal.backend().buffer()[(sidebar_rect.0, sidebar_rect.1)];
        assert_eq!(sidebar_border_cell_2.fg, Color::DarkGray);

        let code_border_cell_2 = &terminal.backend().buffer()[(code_rect.0, code_rect.1)];
        assert_eq!(code_border_cell_2.fg, Color::Cyan);
    }

    fn main_chunks_0_rect() -> (u16, u16) {
        // Main split is at row y=3 (below header height 3). Left sidebar starts at x=0, y=3
        (0, 3)
    }

    fn main_chunks_1_rect() -> (u16, u16) {
        // Right code viewer starts at x=32, y=3 on a 100-col screen (32% of 100 = 32)
        (32, 3)
    }
}
