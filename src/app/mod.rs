use std::path::PathBuf;

use crate::ui::keymap::{Action, Scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePanel {
    #[default]
    Sidebar,
    CodeViewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarView {
    #[default]
    FileExplorer,
    ModifiedFiles,
    CommitTimeline,
}

impl SidebarView {
    pub fn name(&self) -> &'static str {
        match self {
            SidebarView::FileExplorer => "1: Explorer",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationMode {
    #[default]
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
    pub repo_path: PathBuf,
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
    pub cursor_line: usize, // 1-based index

    pub status_message: String,
    pub running: bool,
}

impl AppState {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            active_panel: ActivePanel::Sidebar,
            sidebar_view: SidebarView::FileExplorer,
            nav_mode: NavigationMode::File,

            files: Vec::new(),
            file_selected: 0,

            modified_files: Vec::new(),
            modified_selected: 0,

            commits: Vec::new(),
            commit_selected: 0,

            code_lines: Vec::new(),
            cursor_line: 1,

            status_message: "Press 'Tab' or 'h'/'l' to switch focus. 'm' to change nav mode."
                .to_string(),
            running: true,
        }
    }

    pub fn toggle_panel_focus(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Sidebar => ActivePanel::CodeViewer,
            ActivePanel::CodeViewer => ActivePanel::Sidebar,
        };
        self.status_message = format!("Switched focus to {:?}", self.active_panel);
    }

    pub fn set_sidebar_view(&mut self, view: SidebarView) {
        self.sidebar_view = view;
        self.status_message = format!("Sidebar view: {}", view.name());
    }

    pub fn cycle_navigation_mode(&mut self) {
        self.nav_mode = self.nav_mode.cycle();
        self.status_message = format!("Navigation mode: {}", self.nav_mode.name());
    }

    pub fn set_navigation_mode(&mut self, mode: NavigationMode) {
        self.nav_mode = mode;
        self.status_message = format!("Navigation mode: {}", mode.name());
    }

    pub fn move_selection_down(&mut self) {
        match self.active_panel {
            ActivePanel::Sidebar => match self.sidebar_view {
                SidebarView::FileExplorer => {
                    if !self.files.is_empty() && self.file_selected + 1 < self.files.len() {
                        self.file_selected += 1;
                    }
                }
                SidebarView::ModifiedFiles => {
                    if !self.modified_files.is_empty()
                        && self.modified_selected + 1 < self.modified_files.len()
                    {
                        self.modified_selected += 1;
                    }
                }
                SidebarView::CommitTimeline => {
                    if !self.commits.is_empty() && self.commit_selected + 1 < self.commits.len() {
                        self.commit_selected += 1;
                    }
                }
            },
            ActivePanel::CodeViewer => {
                if !self.code_lines.is_empty() && self.cursor_line < self.code_lines.len() {
                    self.cursor_line += 1;
                }
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        match self.active_panel {
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
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn active_scope(&self) -> Scope {
        match self.active_panel {
            ActivePanel::Sidebar => Scope::Sidebar,
            ActivePanel::CodeViewer => Scope::CodeViewer,
        }
    }

    pub fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit(),
            Action::ToggleFocus => self.toggle_panel_focus(),
            Action::SetSidebarView(index) => match index {
                1 => self.set_sidebar_view(SidebarView::FileExplorer),
                2 => self.set_sidebar_view(SidebarView::ModifiedFiles),
                3 => self.set_sidebar_view(SidebarView::CommitTimeline),
                _ => {}
            },
            Action::CycleNavMode => self.cycle_navigation_mode(),
            Action::MoveUp => self.move_selection_up(),
            Action::MoveDown => self.move_selection_down(),
            Action::Select => {
                match self.sidebar_view {
                    SidebarView::FileExplorer => {
                        if let Some(f) = self.files.get(self.file_selected).cloned() {
                            let file_path = self.repo_path.join(&f);
                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                self.code_lines =
                                    content.lines().map(|s| s.to_string()).collect();
                                self.cursor_line = 1;
                                self.status_message = format!("Loaded file: {}", f);
                            } else {
                                self.status_message = format!("Could not read file: {}", f);
                            }
                        } else {
                            self.status_message = "No file selected".to_string();
                        }
                    }
                    SidebarView::ModifiedFiles => {
                        if let Some(item) =
                            self.modified_files.get(self.modified_selected).cloned()
                        {
                            let clean_path = item.split_whitespace().next().unwrap_or(&item);
                            let file_path = self.repo_path.join(clean_path);
                            if let Ok(content) = std::fs::read_to_string(&file_path) {
                                self.code_lines =
                                    content.lines().map(|s| s.to_string()).collect();
                                self.cursor_line = 1;
                                self.status_message =
                                    format!("Loaded modified file: {}", clean_path);
                            } else {
                                self.status_message =
                                    format!("Could not read file: {}", clean_path);
                            }
                        } else {
                            self.status_message = "No modified file selected".to_string();
                        }
                    }
                    SidebarView::CommitTimeline => {
                        if let Some((hash, msg)) = self.commits.get(self.commit_selected) {
                            self.status_message = format!("Selected commit: {} {}", hash, msg);
                        } else {
                            "No commit selected".to_string();
                        }
                    }
                }
            }
            Action::JumpNextAuto => {
                self.status_message = format!(
                    "Jumping NEXT in [{}] for line {}",
                    self.nav_mode.name(),
                    self.cursor_line
                );
            }
            Action::JumpPrevAuto => {
                self.status_message = format!(
                    "Jumping PREVIOUS in [{}] for line {}",
                    self.nav_mode.name(),
                    self.cursor_line
                );
            }
            Action::JumpNextFile => {
                self.status_message = format!(
                    "Jumping NEXT in [FILE Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::JumpPrevFile => {
                self.status_message = format!(
                    "Jumping PREVIOUS in [FILE Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::JumpNextFunction => {
                self.status_message = format!(
                    "Jumping NEXT in [FUNCTION Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::JumpPrevFunction => {
                self.status_message = format!(
                    "Jumping PREVIOUS in [FUNCTION Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::JumpNextLine => {
                self.status_message = format!(
                    "Jumping NEXT in [LINE Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::JumpPrevLine => {
                self.status_message = format!(
                    "Jumping PREVIOUS in [LINE Mode] for line {}",
                    self.cursor_line
                );
            }
            Action::InlineRewrite => {
                self.status_message = "Triggered Inline Rewrite".to_string();
            }
            Action::EditHere => {
                self.status_message = "Triggered Edit Here".to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        let app = AppState::new(PathBuf::from("/test/repo"));
        assert_eq!(app.repo_path, PathBuf::from("/test/repo"));
        assert_eq!(app.active_panel, ActivePanel::Sidebar);
        assert_eq!(app.sidebar_view, SidebarView::FileExplorer);
        assert_eq!(app.nav_mode, NavigationMode::File);
        assert!(app.running);
    }

    #[test]
    fn test_panel_focus_toggle() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.active_panel, ActivePanel::Sidebar);

        app.toggle_panel_focus();
        assert_eq!(app.active_panel, ActivePanel::CodeViewer);

        app.toggle_panel_focus();
        assert_eq!(app.active_panel, ActivePanel::Sidebar);
    }

    #[test]
    fn test_navigation_mode_cycle() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.nav_mode, NavigationMode::File);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::Function);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::Line);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::File);
    }

    #[test]
    fn test_sidebar_view_transitions() {
        let mut app = AppState::new(PathBuf::from("."));
        app.set_sidebar_view(SidebarView::ModifiedFiles);
        assert_eq!(app.sidebar_view, SidebarView::ModifiedFiles);

        app.set_sidebar_view(SidebarView::CommitTimeline);
        assert_eq!(app.sidebar_view, SidebarView::CommitTimeline);
    }

    #[test]
    fn test_selection_bounds() {
        let mut app = AppState::new(PathBuf::from("."));
        app.files = vec!["file1.rs".into(), "file2.rs".into()];

        // Bounds check down
        app.move_selection_down();
        assert_eq!(app.file_selected, 1);

        app.move_selection_down();
        assert_eq!(app.file_selected, 1); // Clamp at len - 1

        // Bounds check up
        app.move_selection_up();
        assert_eq!(app.file_selected, 0);

        app.move_selection_up();
        assert_eq!(app.file_selected, 0); // Clamp at 0
    }
}
