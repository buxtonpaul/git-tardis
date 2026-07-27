use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

use crate::app::{ActivePanel, AppState, SidebarView};
use crate::treesitter::{capture_name_to_style, highlight_viewport};

pub fn render(frame: &mut Frame, state: &mut AppState) {
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

    let tab2_title = match &state.selected_commit_hash {
        Some(hash) => format!("2: Modified [{}]", &hash[..7.min(hash.len())]),
        None => "2: Dirty Files".to_string(),
    };

    let timeline_title = format!("3: Commit Timeline [{}]", state.timeline_filter.name());

    let sidebar_titles = vec![
        SidebarView::FileExplorer.name().to_string(),
        tab2_title.clone(),
        timeline_title.clone(),
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
    let sidebar_title_name = match state.sidebar_view {
        SidebarView::FileExplorer => SidebarView::FileExplorer.name().to_string(),
        SidebarView::ModifiedFiles => tab2_title,
        SidebarView::CommitTimeline => timeline_title,
    };

    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Sidebar [{}] ", sidebar_title_name))
        .border_style(Style::default().fg(sidebar_border_color));

    match state.sidebar_view {
        SidebarView::FileExplorer => {
            let visible_items = state.visible_file_items();
            let sel_index = if visible_items.is_empty() {
                0
            } else {
                state.file_selected.min(visible_items.len() - 1)
            };
            let items: Vec<ListItem> = visible_items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let indent = "  ".repeat(item.depth);
                    let prefix = if item.is_dir {
                        if item.is_expanded {
                            "▼ "
                        } else {
                            "▶ "
                        }
                    } else {
                        "  "
                    };

                    let is_selected = i == sel_index;
                    let is_active_item = is_selected && is_sidebar_active;

                    if item.is_dir {
                        let has_modified = state.dir_has_modified_files(&item.path);
                        let name_style = if is_active_item {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else if is_selected || has_modified {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        };

                        let mut spans = vec![
                            Span::styled(
                                format!("{}{}", indent, prefix),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(item.name.clone(), name_style),
                        ];

                        if has_modified {
                            let badge_style = if is_active_item {
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            };
                            spans.push(Span::styled(" [*]", badge_style));
                        }

                        let item_style = if is_active_item {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };

                        ListItem::new(Line::from(spans)).style(item_style)
                    } else {
                        let status_code = state.file_modified_status(&item.path).unwrap_or("");
                        let file_color = match status_code {
                            "M" | " M" | "M " => Color::Yellow,
                            "A" | " A" | "A " | "?" | "??" => Color::Green,
                            "D" | " D" | "D " => Color::Red,
                            "R" | "C" | "U" => Color::Magenta,
                            _ if !status_code.is_empty() => Color::Yellow,
                            _ => Color::Reset,
                        };

                        let name_style = if is_active_item {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else if is_selected {
                            Style::default().fg(Color::Yellow)
                        } else if !status_code.is_empty() {
                            Style::default().fg(file_color).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        let mut spans = vec![
                            Span::raw(format!("{}{}", indent, prefix)),
                            Span::styled(item.name.clone(), name_style),
                        ];

                        if !status_code.is_empty() {
                            let badge = format!(" [{}]", status_code.trim());
                            let badge_style = if is_active_item {
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(file_color).add_modifier(Modifier::BOLD)
                            };
                            spans.push(Span::styled(badge, badge_style));
                        }

                        let item_style = if is_active_item {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };

                        ListItem::new(Line::from(spans)).style(item_style)
                    }
                })
                .collect();
            let list = List::new(items).block(sidebar_block);
            let mut list_state = ListState::default();
            if !visible_items.is_empty() {
                list_state.select(Some(sel_index));
            }
            frame.render_stateful_widget(list, main_chunks[0], &mut list_state);
        }
        SidebarView::ModifiedFiles => {
            let (list_items, sel_index) = if state.selected_commit_hash.is_some() {
                (&state.modified_files, state.modified_selected)
            } else {
                (&state.dirty_files, state.dirty_selected)
            };
            let safe_sel = if list_items.is_empty() {
                0
            } else {
                sel_index.min(list_items.len() - 1)
            };

            let items: Vec<ListItem> = list_items
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let style = if i == safe_sel && is_sidebar_active {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else if i == safe_sel {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(f.display_string()).style(style)
                })
                .collect();
            let list = List::new(items).block(sidebar_block);
            let mut list_state = ListState::default();
            if !list_items.is_empty() {
                list_state.select(Some(safe_sel));
            }
            frame.render_stateful_widget(list, main_chunks[0], &mut list_state);
        }
        SidebarView::CommitTimeline => {
            if state.timeline_filter == crate::app::TimelineFilter::All {
                let safe_sel = if state.commits.is_empty() {
                    0
                } else {
                    state.commit_selected.min(state.commits.len() - 1)
                };

                let items: Vec<ListItem> = state
                    .commits
                    .iter()
                    .enumerate()
                    .map(|(i, commit)| {
                        let is_candidate = state
                            .candidate_commits
                            .iter()
                            .any(|cand| commit.matches_candidate(cand));

                        let prefix = if is_candidate { "* " } else { "  " };
                        let display_hash = if !commit.short_hash.is_empty() {
                            &commit.short_hash
                        } else if commit.hash.len() >= 7 {
                            &commit.hash[..7]
                        } else {
                            &commit.hash
                        };
                        let text = format!("{}{} {}", prefix, display_hash, commit.message);

                        let style = if i == safe_sel && is_sidebar_active {
                            Style::default()
                                .bg(Color::Blue)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else if i == safe_sel {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else if is_candidate {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        ListItem::new(text).style(style)
                    })
                    .collect();
                let list = List::new(items).block(sidebar_block);
                let mut list_state = ListState::default();
                if !state.commits.is_empty() {
                    list_state.select(Some(safe_sel));
                }
                frame.render_stateful_widget(list, main_chunks[0], &mut list_state);
            } else {
                let safe_sel = if state.candidate_commits.is_empty() {
                    0
                } else {
                    state
                        .candidate_selected
                        .min(state.candidate_commits.len() - 1)
                };

                let items: Vec<ListItem> = state
                    .candidate_commits
                    .iter()
                    .enumerate()
                    .map(|(i, commit)| {
                        let display_hash = if !commit.short_hash.is_empty() {
                            &commit.short_hash
                        } else if commit.hash.len() >= 7 {
                            &commit.hash[..7]
                        } else {
                            &commit.hash
                        };
                        let text = format!("* {} {}", display_hash, commit.message);
                        let style = if i == safe_sel && is_sidebar_active {
                            Style::default().bg(Color::Blue).fg(Color::White)
                        } else if i == safe_sel {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Cyan)
                        };
                        ListItem::new(text).style(style)
                    })
                    .collect();
                let list = List::new(items).block(sidebar_block);
                let mut list_state = ListState::default();
                if !state.candidate_commits.is_empty() {
                    list_state.select(Some(safe_sel));
                }
                frame.render_stateful_widget(list, main_chunks[0], &mut list_state);
            }
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
            " Code Viewer - Mode: [{}] | View: [{}] | Blame: {} ",
            state.nav_mode.name(),
            state.file_view_mode.name(),
            blame_str
        )
    } else {
        format!(
            " Code Viewer - Mode: [{}] | View: [{}] ",
            state.nav_mode.name(),
            state.file_view_mode.name()
        )
    };

    let code_block = Block::default()
        .borders(Borders::ALL)
        .title(title_text)
        .border_style(Style::default().fg(code_border_color));

    let mut formatted_code = Vec::new();
    if state.code_lines.is_empty() {
        formatted_code = crate::ui::splash::render_splashscreen_lines(state.git_version.as_deref());
    } else {
        let cur_path = state.current_file_path();
        let is_markdown = crate::ui::markdown::is_markdown_file(cur_path.as_deref())
            && state.render_markdown_formatted;

        let grammar_entry = if !is_markdown {
            cur_path.and_then(|path| {
                let ext = std::path::Path::new(&path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                state.grammar_registry.get_by_extension(ext)
            })
        } else {
            None
        };

        let source_code = state.code_lines.join("\n");
        let highlighted_lines = grammar_entry
            .as_ref()
            .map(|entry| highlight_viewport(entry, &source_code, 1, state.code_lines.len()));

        let mut md_state = crate::ui::markdown::MarkdownFormatterState::new();

        for (idx, line) in state.code_lines.iter().enumerate() {
            let line_num = idx + 1;
            let is_cursor = line_num == state.cursor_line;

            let line_diff_type = if state.file_view_mode == crate::app::FileViewMode::Diff {
                crate::git::diff_parser::classify_diff_line(line)
            } else {
                state
                    .file_diff_highlights
                    .get(&line_num)
                    .copied()
                    .unwrap_or(crate::git::DiffLineType::Context)
            };

            let (diff_prefix, diff_color) = match line_diff_type {
                crate::git::DiffLineType::Added => ("+ ", Some(Color::Green)),
                crate::git::DiffLineType::Modified => ("~ ", Some(Color::Yellow)),
                crate::git::DiffLineType::Deleted => ("- ", Some(Color::Red)),
                crate::git::DiffLineType::HunkHeader => ("@@", Some(Color::Cyan)),
                crate::git::DiffLineType::DiffHeader => ("##", Some(Color::Yellow)),
                crate::git::DiffLineType::Context => ("  ", None),
            };

            let prefix = if is_cursor { "> " } else { diff_prefix };

            let gutter_style = if is_cursor && is_code_active {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_cursor {
                Style::default().fg(Color::Yellow)
            } else if let Some(col) = diff_color {
                Style::default().fg(col)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let gutter_text = format!("{:>3} {}", line_num, prefix);
            let mut spans = vec![Span::styled(gutter_text, gutter_style)];

            if is_markdown {
                let base_style = if is_cursor && is_code_active {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if is_cursor {
                    Style::default().fg(Color::Yellow)
                } else {
                    match line_diff_type {
                        crate::git::DiffLineType::Added => Style::default().fg(Color::Green),
                        crate::git::DiffLineType::Modified => Style::default().fg(Color::Yellow),
                        crate::git::DiffLineType::Deleted => Style::default().fg(Color::Red),
                        crate::git::DiffLineType::HunkHeader => Style::default().fg(Color::Cyan),
                        crate::git::DiffLineType::DiffHeader => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        crate::git::DiffLineType::Context => Style::default(),
                    }
                };

                let md_spans =
                    crate::ui::markdown::render_markdown_line(line, &mut md_state, base_style);
                spans.extend(md_spans);
            } else if let Some(hl_line) = highlighted_lines.as_ref().and_then(|hl| hl.get(idx)) {
                for hl_span in &hl_line.spans {
                    let mut span_style = capture_name_to_style(&hl_span.capture_name);
                    if is_cursor && is_code_active {
                        span_style = span_style.bg(Color::DarkGray);
                        if span_style.fg.is_none() {
                            span_style = span_style.fg(Color::Yellow);
                        }
                        span_style = span_style.add_modifier(Modifier::BOLD);
                    } else if is_cursor && span_style.fg.is_none() {
                        span_style = span_style.fg(Color::Yellow);
                    } else if !is_cursor {
                        match line_diff_type {
                            crate::git::DiffLineType::Added => {
                                if span_style.fg.is_none() {
                                    span_style = span_style.fg(Color::Green);
                                }
                            }
                            crate::git::DiffLineType::Modified => {
                                if span_style.fg.is_none() {
                                    span_style = span_style.fg(Color::Yellow);
                                }
                            }
                            crate::git::DiffLineType::Deleted => {
                                if span_style.fg.is_none() {
                                    span_style = span_style.fg(Color::Red);
                                }
                            }
                            crate::git::DiffLineType::HunkHeader => {
                                span_style = span_style.fg(Color::Cyan);
                            }
                            crate::git::DiffLineType::DiffHeader => {
                                span_style =
                                    span_style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                            }
                            crate::git::DiffLineType::Context => {}
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
                    match line_diff_type {
                        crate::git::DiffLineType::Added => Style::default().fg(Color::Green),
                        crate::git::DiffLineType::Modified => Style::default().fg(Color::Yellow),
                        crate::git::DiffLineType::Deleted => Style::default().fg(Color::Red),
                        crate::git::DiffLineType::HunkHeader => Style::default().fg(Color::Cyan),
                        crate::git::DiffLineType::DiffHeader => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        crate::git::DiffLineType::Context => Style::default(),
                    }
                };
                spans.push(Span::styled(line.clone(), line_style));
            };

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
    state.sidebar_viewport_height = main_chunks[0].height.saturating_sub(2) as usize;
    state.ensure_cursor_visible(viewport_height);
    let scroll_offset = state.code_scroll_offset;

    let paragraph = Paragraph::new(formatted_code)
        .block(code_block)
        .scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, main_chunks[1]);

    // 3. Footer Status Bar
    let status_text = format!(
        " Status: {} | Keys: [?] Help | [S] Splash | [Tab] Switch Panel | [1/2/3] Sidebar View | [c] Filter | [m] Nav Mode | [q] Quit",
        state.status_message
    );
    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Green));
    frame.render_widget(status_bar, chunks[2]);

    // 4. Keybindings Help Overlay Popup
    if state.show_help {
        let area = centered_rect(85, 95, frame.area());
        frame.render_widget(Clear, area);

        let help_block = Block::default()
            .borders(Borders::ALL)
            .title(" Keybindings Help (?) ")
            .border_style(Style::default().fg(Color::Yellow));

        let help_lines = vec![
            Line::from(vec![Span::styled(
                "--- Global Shortcuts ---",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ?               Toggle Keybindings Help Screen"),
            Line::from("  <Tab> / h / l   Switch Focus between Sidebar and Code Viewer"),
            Line::from(
                "  1 / 2 / 3       Switch Sidebar View (1: Explorer, 2: Modified, 3: Timeline)",
            ),
            Line::from("  c               Toggle Timeline Filter (ALL vs CANDIDATES)"),
            Line::from("  m               Cycle Navigation Mode (Commit, File, Function, Line)"),
            Line::from("  d               Toggle File Viewer Mode (FULL contents vs DIFF)"),
            Line::from("  M               Toggle Formatted Markdown View vs Raw Text"),
            Line::from("  q / <Esc>       Close Help / Reset Time Travel / Exit Application"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "--- Sidebar Navigation ---",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  j / k / Down / Up   Move selection down / up"),
            Line::from("  <CR> / l            Select file / Toggle folder expansion"),
            Line::from("  <Right>             Expand folder / Step down to child"),
            Line::from("  <Left> / h          Collapse folder / Jump to parent folder"),
            Line::from("  <Space>             Toggle folder fold/unfold"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "--- Code Viewer & Time Travel ---",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  j / k / Down / Up   Move line cursor down / up"),
            Line::from("  ] / [               Jump to Next / Previous historical commit"),
            Line::from("  <C-d> / <C-u>       Half page scroll down / up"),
            Line::from("  <C-f> / <C-b>       Page scroll down / up"),
            Line::from("  <C-e> / <C-y>       Scroll single line down / up"),
            Line::from("  zz / zt / zb        Center cursor / Cursor top / Cursor bottom"),
            Line::from("  :                   Go to Line Number"),
            Line::from("  /                   Search Text Pattern in File"),
            Line::from("  s                   Search Symbol (Tree-sitter AST locator)"),
            Line::from("  n / N               Jump to Next / Previous search match"),
            Line::from("  e / E               Trigger Inline Rewrite / Edit Here"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press '?' or 'Esc' to close this help window",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]),
        ];

        let help_paragraph = Paragraph::new(help_lines).block(help_block);
        frame.render_widget(help_paragraph, area);
    }

    // 5. TARDIS Splashscreen Overlay Popup
    if state.show_splashscreen {
        let area = centered_rect(80, 88, frame.area());
        frame.render_widget(Clear, area);

        let splash_block = Block::default()
            .borders(Borders::ALL)
            .title(" TARDIS Splashscreen / About ")
            .border_style(Style::default().fg(Color::Cyan));

        let splash_lines =
            crate::ui::splash::render_splashscreen_lines(state.git_version.as_deref());
        let splash_paragraph = Paragraph::new(splash_lines).block(splash_block);
        frame.render_widget(splash_paragraph, area);
    }

    // 6. Navigation Input Prompt Overlay Modal
    if let Some(prompt) = &state.input_prompt {
        let area = centered_rect(65, 45, frame.area());
        frame.render_widget(Clear, area);

        let (title, prefix) = match prompt {
            crate::app::InputPrompt::GotoLine => (" Go to Line ", ": "),
            crate::app::InputPrompt::SearchText => (" Search Text ", "/ "),
            crate::app::InputPrompt::SearchSymbol => (" Search Symbol (Tree-sitter) ", "Symbol: "),
        };

        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::Yellow));

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&state.input_buffer, Style::default().fg(Color::Yellow)),
                Span::styled("█", Style::default().fg(Color::Green)),
            ]),
            Line::from(""),
        ];

        if *prompt == crate::app::InputPrompt::SearchSymbol {
            let filtered = state.filtered_symbols(&state.input_buffer);
            if filtered.is_empty() {
                lines.push(Line::from(Span::styled(
                    "  No matching symbols",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            } else {
                for (idx, item) in filtered.iter().take(10).enumerate() {
                    let is_sel = idx == state.symbol_selected;
                    let style = if is_sel {
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "  {:>6}  line {:<4}  {}",
                            item.kind, item.line_number, item.name
                        ),
                        style,
                    )]));
                }
            }
        } else if *prompt == crate::app::InputPrompt::SearchText && !state.search_matches.is_empty()
        {
            lines.push(Line::from(Span::styled(
                format!("  {} line matches found", state.search_matches.len()),
                Style::default().fg(Color::Green),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press <Enter> to Jump | <Esc> to Cancel",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));

        let prompt_paragraph = Paragraph::new(lines).block(prompt_block);
        frame.render_widget(prompt_paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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
        app.expand_all_folders();
        app.code_lines = vec!["fn main() {}".into()];

        terminal.draw(|f| render(f, &mut app)).unwrap();

        let buffer = terminal.backend().buffer();
        let content = format!("{:?}", buffer);

        // Check essential UI titles and sections
        assert!(content.contains("Git-tardis TUI"));
        assert!(content.contains("1: Explorer"));
        assert!(content.contains("2: Dirty Files"));
        assert!(content.contains("3: Commit Timeline"));
        assert!(content.contains("Sidebar [1: Explorer]"));
        assert!(content.contains("Code Viewer - Mode: [COMMIT Mode]"));
        assert!(content.contains("src/"));
        assert!(content.contains("main.rs"));
        assert!(content.contains("fn main() {}"));
    }

    #[test]
    fn test_border_color_focus_toggle() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.files = vec!["src/main.rs".into()];

        // Render with Sidebar active
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let sidebar_rect = main_chunks_0_rect(); // top left panel
        let code_rect = main_chunks_1_rect();

        // Cell at top-left of sidebar block should be Cyan when active
        let sidebar_border_cell = &terminal.backend().buffer()[(sidebar_rect.0, sidebar_rect.1)];
        assert_eq!(sidebar_border_cell.fg, Color::Cyan);

        let code_border_cell = &terminal.backend().buffer()[(code_rect.0, code_rect.1)];
        assert_eq!(code_border_cell.fg, Color::DarkGray);

        // Switch focus to CodeViewer
        app.toggle_panel_focus();
        terminal.draw(|f| render(f, &mut app)).unwrap();

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
