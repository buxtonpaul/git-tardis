use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use std::io::stdout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Sidebar,
    CodeViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarView {
    FileExplorer,
    ModifiedFiles,
    CommitTimeline,
}

impl SidebarView {
    pub fn name(&self) -> &'static str {
        match self {
            SidebarView::FileExplorer => "1: File Explorer",
            SidebarView::ModifiedFiles => "2: Modified Files",
            SidebarView::CommitTimeline => "3: Commit Timeline",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SidebarView::FileExplorer => SidebarView::ModifiedFiles,
            SidebarView::ModifiedFiles => SidebarView::CommitTimeline,
            SidebarView::CommitTimeline => SidebarView::FileExplorer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMode {
    File,
    Function,
    Line,
}

impl NavigationMode {
    pub fn name(&self) -> &'static str {
        match self {
            NavigationMode::File => "FILE Mode",
            NavigationMode::Function => "FUNCTION Mode",
            NavigationMode::Line => "LINE Mode",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            NavigationMode::File => NavigationMode::Function,
            NavigationMode::Function => NavigationMode::Line,
            NavigationMode::Line => NavigationMode::File,
        }
    }
}

pub struct AppState {
    pub active_panel: ActivePanel,
    pub sidebar_view: SidebarView,
    pub nav_mode: NavigationMode,

    pub files: Vec<String>,
    pub file_selected: usize,

    pub modified_files: Vec<String>,
    pub modified_selected: usize,

    pub commits: Vec<(String, String)>,
    pub commit_selected: usize,

    pub code_lines: Vec<String>,
    pub cursor_line: usize,

    pub status_message: String,
    pub running: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_panel: ActivePanel::Sidebar,
            sidebar_view: SidebarView::FileExplorer,
            nav_mode: NavigationMode::File,

            files: vec![
                "src/main.rs".into(),
                "src/parser.rs".into(),
                "src/tui/app.rs".into(),
                "src/tui/sidebar.rs".into(),
                "Cargo.toml".into(),
                "README.md".into(),
            ],
            file_selected: 0,

            modified_files: vec![
                "src/parser.rs (M)".into(),
                "src/tui/sidebar.rs (M)".into(),
            ],
            modified_selected: 0,

            commits: vec![
                ("a1b2c3d".into(), "feat: Add tree-sitter function boundary detection".into()),
                ("e5f6g7h".into(), "fix: Handle leading indentation in Python".into()),
                ("i9j0k1l".into(), "refactor: Add async Neovim RPC client".into()),
                ("m2n3o4p".into(), "docs: Initial Git-tardis design spec".into()),
            ],
            commit_selected: 0,

            code_lines: vec![
                "// Welcome to Git-tardis TUI Prototype!".into(),
                "fn main() {".into(),
                "    let mut app = AppState::new();".into(),
                "    println!(\"Starting application loop...\");".into(),
                "    app.run();".into(),
                "}".into(),
                "".into(),
                "pub fn parse_enclosing_function(line: usize) -> Option<Range> {".into(),
                "    // Locates function range around cursor line".into(),
                "    let point = Point { row: line - 1, column: 0 };".into(),
                "    let node = root.descendant_for_point_range(point, point)?;".into(),
                "    Some(node.range())".into(),
                "}".into(),
            ],
            cursor_line: 2, // 1-based

            status_message: "Press 'Tab' or 'h'/'l' to switch panels. '1'/'2'/'3' for sidebar. 'm' to change nav mode.".into(),
            running: true,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            KeyCode::Tab
            | KeyCode::Char('h')
            | KeyCode::Char('l')
            | KeyCode::Left
            | KeyCode::Right => {
                self.active_panel = match self.active_panel {
                    ActivePanel::Sidebar => ActivePanel::CodeViewer,
                    ActivePanel::CodeViewer => ActivePanel::Sidebar,
                };
                self.status_message = format!("Switched focus to: {:?}", self.active_panel);
            }
            KeyCode::Char('1') => {
                self.sidebar_view = SidebarView::FileExplorer;
                self.status_message = "Sidebar view: File Explorer".into();
            }
            KeyCode::Char('2') => {
                self.sidebar_view = SidebarView::ModifiedFiles;
                self.status_message = "Sidebar view: Modified Files".into();
            }
            KeyCode::Char('3') => {
                self.sidebar_view = SidebarView::CommitTimeline;
                self.status_message = "Sidebar view: Commit Timeline".into();
            }
            KeyCode::Char('m') => {
                self.nav_mode = self.nav_mode.cycle();
                self.status_message =
                    format!("Navigation Mode changed to: {}", self.nav_mode.name());
            }
            KeyCode::Char('j') | KeyCode::Down => match self.active_panel {
                ActivePanel::Sidebar => match self.sidebar_view {
                    SidebarView::FileExplorer => {
                        if self.file_selected + 1 < self.files.len() {
                            self.file_selected += 1;
                        }
                    }
                    SidebarView::ModifiedFiles => {
                        if self.modified_selected + 1 < self.modified_files.len() {
                            self.modified_selected += 1;
                        }
                    }
                    SidebarView::CommitTimeline => {
                        if self.commit_selected + 1 < self.commits.len() {
                            self.commit_selected += 1;
                        }
                    }
                },
                ActivePanel::CodeViewer => {
                    if self.cursor_line < self.code_lines.len() {
                        self.cursor_line += 1;
                    }
                }
            },
            KeyCode::Char('k') | KeyCode::Up => match self.active_panel {
                ActivePanel::Sidebar => match self.sidebar_view {
                    SidebarView::FileExplorer => {
                        if self.file_selected > 0 {
                            self.file_selected -= 1;
                        }
                    }
                    SidebarView::ModifiedFiles => {
                        if self.modified_selected > 0 {
                            self.modified_selected -= 1;
                        }
                    }
                    SidebarView::CommitTimeline => {
                        if self.commit_selected > 0 {
                            self.commit_selected -= 1;
                        }
                    }
                },
                ActivePanel::CodeViewer => {
                    if self.cursor_line > 1 {
                        self.cursor_line -= 1;
                    }
                }
            },
            KeyCode::Char('n') | KeyCode::Char(']') => {
                self.status_message = format!(
                    "Jumping NEXT in [{}] for line {}",
                    self.nav_mode.name(),
                    self.cursor_line
                );
            }
            KeyCode::Char('p') | KeyCode::Char('[') => {
                self.status_message = format!(
                    "Jumping PREVIOUS in [{}] for line {}",
                    self.nav_mode.name(),
                    self.cursor_line
                );
            }
            _ => {}
        }
    }
}

fn ui(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(0),    // Main content split
            Constraint::Length(3), // Footer status bar
        ])
        .split(frame.area());

    // 1. Top Bar / Title
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

    // 2. Middle Main Area: Horizontal Split
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(32), // Left Sidebar
            Constraint::Percentage(68), // Right Code Viewer
        ])
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
    let code_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Code Viewer - Mode: [{}] ", state.nav_mode.name()))
        .border_style(Style::default().fg(code_border_color));

    let mut formatted_code = Vec::new();
    for (idx, line) in state.code_lines.iter().enumerate() {
        let line_num = idx + 1;
        let is_cursor = line_num == state.cursor_line;
        let prefix = if is_cursor { "> " } else { "  " };
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

        let content = format!("{:>3} {}{}", line_num, prefix, line);
        formatted_code.push(Line::from(Span::styled(content, line_style)));
    }

    let paragraph = Paragraph::new(formatted_code).block(code_block);
    frame.render_widget(paragraph, main_chunks[1]);

    // 3. Footer Status Bar
    let status_text = format!(
        " Status: {} | Keys: [Tab] Switch Panel | [1/2/3] Sidebar View | [m] Nav Mode | [n/p] Jump | [q] Quit",
        state.status_message
    );
    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL).title(" Controls "))
        .style(Style::default().fg(Color::Green));
    frame.render_widget(status_bar, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut state = AppState::new();

    while state.running {
        terminal.draw(|f| ui(f, &state))?;

        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            state.handle_key(key.code);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
