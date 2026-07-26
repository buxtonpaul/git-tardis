use std::path::PathBuf;

use crate::git::BlameLine;
use crate::treesitter::GrammarRegistry;
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
    TargetCandidates,
}

impl SidebarView {
    pub fn name(&self) -> &'static str {
        match self {
            SidebarView::FileExplorer => "1: Explorer",
            SidebarView::ModifiedFiles => "2: Modified Files",
            SidebarView::CommitTimeline => "3: Commit Timeline",
            SidebarView::TargetCandidates => "4: Candidates",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SidebarView::FileExplorer => SidebarView::ModifiedFiles,
            SidebarView::ModifiedFiles => SidebarView::CommitTimeline,
            SidebarView::CommitTimeline => SidebarView::TargetCandidates,
            SidebarView::TargetCandidates => SidebarView::FileExplorer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationMode {
    #[default]
    Commit,
    File,
    Function,
    Line,
}

impl NavigationMode {
    pub fn name(&self) -> &'static str {
        match self {
            NavigationMode::Commit => "COMMIT Mode",
            NavigationMode::File => "FILE Mode",
            NavigationMode::Function => "FUNCTION Mode",
            NavigationMode::Line => "LINE Mode",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            NavigationMode::Commit => NavigationMode::File,
            NavigationMode::File => NavigationMode::Function,
            NavigationMode::Function => NavigationMode::Line,
            NavigationMode::Line => NavigationMode::Commit,
        }
    }
}

pub struct AppState {
    pub repo_path: PathBuf,
    pub active_panel: ActivePanel,
    pub sidebar_view: SidebarView,
    pub nav_mode: NavigationMode,
    pub active_file: Option<String>,

    pub files: Vec<String>,
    pub file_selected: usize,

    pub dirty_files: Vec<String>,
    pub dirty_selected: usize,

    pub modified_files: Vec<String>,
    pub modified_selected: usize,

    pub uncommitted_files: Vec<String>,

    pub commits: Vec<(String, String)>,
    pub commit_selected: usize,
    pub candidate_commits: Vec<(String, String)>,
    pub candidate_selected: usize,
    pub selected_commit_hash: Option<String>,

    pub code_lines: Vec<String>,
    pub cursor_line: usize,        // 1-based index
    pub code_scroll_offset: usize, // 0-based top visible line index
    pub code_viewport_height: usize,
    pub sidebar_viewport_height: usize,
    pub scrolloff: usize,

    pub status_message: String,
    pub running: bool,

    pub grammar_registry: GrammarRegistry,
    pub current_line_blame: Option<BlameLine>,
    pub file_diff_highlights: std::collections::HashMap<usize, crate::git::DiffLineType>,
}

impl AppState {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            active_panel: ActivePanel::Sidebar,
            sidebar_view: SidebarView::FileExplorer,
            nav_mode: NavigationMode::Commit,
            active_file: None,

            files: Vec::new(),
            file_selected: 0,

            dirty_files: Vec::new(),
            dirty_selected: 0,

            modified_files: Vec::new(),
            modified_selected: 0,

            uncommitted_files: Vec::new(),

            commits: Vec::new(),
            commit_selected: 0,
            candidate_commits: Vec::new(),
            candidate_selected: 0,
            selected_commit_hash: None,

            code_lines: Vec::new(),
            cursor_line: 1,
            code_scroll_offset: 0,
            code_viewport_height: 20,
            sidebar_viewport_height: 20,
            scrolloff: 3,

            status_message: "Press 'Tab' or 'h'/'l' to switch focus. 'm' to change nav mode."
                .to_string(),
            running: true,

            grammar_registry: GrammarRegistry::new(),
            current_line_blame: None,
            file_diff_highlights: std::collections::HashMap::new(),
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
        if view == SidebarView::TargetCandidates {
            self.update_candidate_commits();
        }
        self.load_currently_selected_file();
        self.status_message = format!("Sidebar view: {}", view.name());
    }

    pub fn cycle_navigation_mode(&mut self) {
        self.nav_mode = self.nav_mode.cycle();
        self.update_candidate_commits();
        self.status_message = format!("Navigation mode: {}", self.nav_mode.name());
    }

    pub fn set_navigation_mode(&mut self, mode: NavigationMode) {
        self.nav_mode = mode;
        self.update_candidate_commits();
        self.status_message = format!("Navigation mode: {}", mode.name());
    }

    pub fn move_selection_down(&mut self) {
        match self.active_panel {
            ActivePanel::Sidebar => match self.sidebar_view {
                SidebarView::FileExplorer => {
                    if !self.files.is_empty() && self.file_selected + 1 < self.files.len() {
                        self.file_selected += 1;
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::ModifiedFiles => {
                    if self.selected_commit_hash.is_some() {
                        if !self.modified_files.is_empty()
                            && self.modified_selected + 1 < self.modified_files.len()
                        {
                            self.modified_selected += 1;
                            self.load_currently_selected_file();
                        }
                    } else if !self.dirty_files.is_empty()
                        && self.dirty_selected + 1 < self.dirty_files.len()
                    {
                        self.dirty_selected += 1;
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::CommitTimeline => {
                    if !self.commits.is_empty() && self.commit_selected + 1 < self.commits.len() {
                        self.commit_selected += 1;
                        self.update_modified_files_for_selected_commit();
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::TargetCandidates => {
                    if !self.candidate_commits.is_empty()
                        && self.candidate_selected + 1 < self.candidate_commits.len()
                    {
                        self.candidate_selected += 1;
                        let (hash, _) = self.candidate_commits[self.candidate_selected].clone();
                        self.update_state_for_commit_hash(hash);
                        self.load_currently_selected_file();
                    }
                }
            },
            ActivePanel::CodeViewer => {
                if !self.code_lines.is_empty() && self.cursor_line < self.code_lines.len() {
                    self.cursor_line += 1;
                    self.update_current_line_blame();
                    self.update_candidate_commits();
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
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::ModifiedFiles => {
                    if self.selected_commit_hash.is_some() {
                        if self.modified_selected > 0 {
                            self.modified_selected -= 1;
                            self.load_currently_selected_file();
                        }
                    } else if self.dirty_selected > 0 {
                        self.dirty_selected -= 1;
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::CommitTimeline => {
                    if self.commit_selected > 0 {
                        self.commit_selected -= 1;
                        self.update_modified_files_for_selected_commit();
                        self.load_currently_selected_file();
                    }
                }
                SidebarView::TargetCandidates => {
                    if self.candidate_selected > 0 {
                        self.candidate_selected -= 1;
                        let (hash, _) = self.candidate_commits[self.candidate_selected].clone();
                        self.update_state_for_commit_hash(hash);
                        self.load_currently_selected_file();
                    }
                }
            },
            ActivePanel::CodeViewer => {
                if self.cursor_line > 1 {
                    self.cursor_line -= 1;
                    self.update_current_line_blame();
                    self.update_candidate_commits();
                }
            }
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn update_candidate_commits(&mut self) {
        let repo = match crate::git::GitRepo::open(&self.repo_path) {
            Ok(r) => r,
            Err(_) => return,
        };

        let cur_file = self.current_file_path();

        let commits_res = match self.nav_mode {
            NavigationMode::Commit => repo.get_commit_history(None),
            NavigationMode::File => {
                if let Some(f) = &cur_file {
                    repo.get_file_commits(f, None)
                } else {
                    repo.get_commit_history(None)
                }
            }
            NavigationMode::Function => {
                if let Some(f) = &cur_file {
                    let source_code = self.code_lines.join("\n");
                    let range = crate::treesitter::scope::find_enclosing_function_range(
                        &self.grammar_registry,
                        f,
                        &source_code,
                        self.cursor_line,
                    );
                    if let Ok(Some((start_l, end_l))) = range {
                        repo.get_line_commits(f, start_l, end_l, None)
                    } else {
                        repo.get_file_commits(f, None)
                    }
                } else {
                    repo.get_commit_history(None)
                }
            }
            NavigationMode::Line => {
                if let Some(f) = &cur_file {
                    repo.get_line_commits(f, self.cursor_line, self.cursor_line, None)
                } else {
                    repo.get_commit_history(None)
                }
            }
        };

        if let Ok(commits) = commits_res {
            self.candidate_commits = commits
                .into_iter()
                .map(|c| (c.short_hash, c.summary))
                .collect();

            if let Some(hash) = &self.selected_commit_hash {
                if let Some(idx) = self
                    .candidate_commits
                    .iter()
                    .position(|(h, _)| h == hash || hash.starts_with(h) || h.starts_with(hash))
                {
                    self.candidate_selected = idx;
                } else {
                    self.candidate_selected = 0;
                }
            } else {
                self.candidate_selected = 0;
            }
        }
    }

    pub fn update_modified_files_for_selected_commit(&mut self) {
        if self.commits.is_empty() || self.commit_selected >= self.commits.len() {
            return;
        }

        let (hash, _) = self.commits[self.commit_selected].clone();
        self.update_state_for_commit_hash(hash);
    }

    pub fn update_state_for_commit_hash(&mut self, hash: String) {
        self.selected_commit_hash = Some(hash.clone());

        // Sync commit_selected index in self.commits if hash exists in commit history
        if let Some(idx) = self
            .commits
            .iter()
            .position(|(h, _)| h == &hash || hash.starts_with(h) || h.starts_with(&hash))
        {
            self.commit_selected = idx;
        }

        if let Some(idx) = self
            .candidate_commits
            .iter()
            .position(|(h, _)| h == &hash || hash.starts_with(h) || h.starts_with(&hash))
        {
            self.candidate_selected = idx;
        }

        // Fetch modified files for this commit
        if let Ok(repo) = crate::git::GitRepo::open(&self.repo_path) {
            if let Ok(commit_files) = repo.get_commit_files(&hash) {
                self.modified_files = commit_files
                    .into_iter()
                    .map(|s| format!("{} ({})", s.path, s.status_code().trim()))
                    .collect();

                // If current file is in modified_files, set modified_selected to match it
                if let Some(cur_file) = self.current_file_path() {
                    if let Some(f_idx) = self.modified_files.iter().position(|item| {
                        let clean = item.split_whitespace().next().unwrap_or(item);
                        clean == cur_file
                    }) {
                        self.modified_selected = f_idx;
                    } else {
                        self.modified_selected = 0;
                    }
                } else {
                    self.modified_selected = 0;
                }
            }
        }

        self.update_file_diff_highlights();
    }

    pub fn reset_time_travel(&mut self) {
        self.selected_commit_hash = None;
        self.load_currently_selected_file();
        self.status_message = "Exited Time-Travel mode (returned to working directory)".to_string();
    }

    pub fn load_currently_selected_file(&mut self) {
        match self.sidebar_view {
            SidebarView::FileExplorer => {
                if let Some(f) = self.files.get(self.file_selected).cloned() {
                    self.active_file = Some(f.clone());
                    let file_path = self.repo_path.join(&f);
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        self.code_lines = content.lines().map(|s| s.to_string()).collect();
                        self.cursor_line = 1;
                        self.code_scroll_offset = 0;
                        self.status_message = format!("Loaded file: {}", f);
                        self.update_current_line_blame();
                    } else {
                        self.code_lines.clear();
                        self.cursor_line = 1;
                        self.status_message = format!("Could not read file: {}", f);
                    }
                } else {
                    self.code_lines.clear();
                    self.cursor_line = 1;
                }
            }
            SidebarView::ModifiedFiles => {
                if let Some(hash) = &self.selected_commit_hash {
                    if let Some(item) = self.modified_files.get(self.modified_selected).cloned() {
                        let clean_path = item.split_whitespace().next().unwrap_or(&item);
                        self.active_file = Some(clean_path.to_string());
                        let is_deleted = item.contains("(D)");

                        if is_deleted {
                            self.code_lines.clear();
                            self.cursor_line = 1;
                            let short_hash = &hash[..7.min(hash.len())];
                            self.status_message = format!(
                                "File '{}' was deleted in commit {}",
                                clean_path, short_hash
                            );
                            self.update_current_line_blame();
                        } else if let Ok(repo) = crate::git::GitRepo::open(&self.repo_path) {
                            if let Ok(content) = repo.get_file_at_commit(hash, clean_path) {
                                self.code_lines = content.lines().map(|s| s.to_string()).collect();
                                self.cursor_line = 1;
                                self.code_scroll_offset = 0;
                                let short_hash = &hash[..7.min(hash.len())];
                                self.status_message =
                                    format!("Loaded {} at commit {}", clean_path, short_hash);
                                self.update_current_line_blame();
                            } else {
                                self.code_lines.clear();
                                self.cursor_line = 1;
                                self.code_scroll_offset = 0;
                                let short_hash = &hash[..7.min(hash.len())];
                                self.status_message = format!(
                                    "Could not read {} at commit {}",
                                    clean_path, short_hash
                                );
                                self.update_current_line_blame();
                            }
                        } else {
                            self.code_lines.clear();
                            self.cursor_line = 1;
                            self.code_scroll_offset = 0;
                            let short_hash = &hash[..7.min(hash.len())];
                            self.status_message =
                                format!("Could not read {} at commit {}", clean_path, short_hash);
                            self.update_current_line_blame();
                        }
                    } else {
                        self.code_lines.clear();
                        self.cursor_line = 1;
                        self.code_scroll_offset = 0;
                        self.update_current_line_blame();
                    }
                } else if let Some(item) = self.dirty_files.get(self.dirty_selected).cloned() {
                    let clean_path = item.split_whitespace().next().unwrap_or(&item);
                    self.active_file = Some(clean_path.to_string());
                    let is_deleted = item.contains("(D)");

                    if is_deleted {
                        self.code_lines.clear();
                        self.cursor_line = 1;
                        self.code_scroll_offset = 0;
                        self.status_message =
                            format!("File '{}' was deleted in working tree", clean_path);
                        self.update_current_line_blame();
                    } else {
                        let file_path = self.repo_path.join(clean_path);
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            self.code_lines = content.lines().map(|s| s.to_string()).collect();
                            self.cursor_line = 1;
                            self.code_scroll_offset = 0;
                            self.status_message = format!("Loaded dirty file: {}", clean_path);
                            self.update_current_line_blame();
                        } else {
                            self.code_lines.clear();
                            self.cursor_line = 1;
                            self.code_scroll_offset = 0;
                            self.status_message = format!("Could not read file: {}", clean_path);
                            self.update_current_line_blame();
                        }
                    }
                } else {
                    self.code_lines.clear();
                    self.cursor_line = 1;
                    self.code_scroll_offset = 0;
                    self.update_current_line_blame();
                }
            }
            SidebarView::CommitTimeline | SidebarView::TargetCandidates => {
                if let Some(hash) = self.selected_commit_hash.clone() {
                    let cur_file = self.current_file_path();
                    if let Some(clean_path) = cur_file {
                        let short_hash = &hash[..7.min(hash.len())];
                        if let Ok(repo) = crate::git::GitRepo::open(&self.repo_path) {
                            match repo.get_file_at_commit(&hash, &clean_path) {
                                Ok(content) => {
                                    self.code_lines =
                                        content.lines().map(|s| s.to_string()).collect();
                                    self.cursor_line = 1;
                                    self.code_scroll_offset = 0;
                                    self.status_message =
                                        format!("Loaded {} at commit {}", clean_path, short_hash);
                                    self.update_current_line_blame();
                                }
                                Err(_) => {
                                    self.code_lines = vec![format!(
                                        "File '{}' did not exist at commit {}",
                                        clean_path, short_hash
                                    )];
                                    self.cursor_line = 1;
                                    self.code_scroll_offset = 0;
                                    self.status_message = format!(
                                        "File '{}' did not exist at commit {}",
                                        clean_path, short_hash
                                    );
                                    self.update_current_line_blame();
                                }
                            }
                        }
                    } else {
                        self.code_lines.clear();
                        self.cursor_line = 1;
                        self.code_scroll_offset = 0;
                        self.update_current_line_blame();
                    }
                }
            }
        }

        self.update_candidate_commits();
        self.update_file_diff_highlights();
    }

    pub fn update_file_diff_highlights(&mut self) {
        self.file_diff_highlights.clear();
        let cur_file = match self.current_file_path() {
            Some(f) => f,
            None => return,
        };

        let diff_res = if let Ok(repo) = crate::git::GitRepo::open(&self.repo_path) {
            if let Some(hash) = &self.selected_commit_hash {
                repo.get_diff_file(hash, &cur_file)
            } else {
                repo.get_working_diff(Some(&cur_file))
            }
        } else {
            return;
        };

        if let Ok(diff_text) = diff_res {
            self.file_diff_highlights = crate::git::diff_parser::parse_file_diff_hunks(&diff_text);
        }
    }

    pub fn current_file_path(&self) -> Option<String> {
        if let Some(f) = &self.active_file {
            return Some(f.clone());
        }
        match self.sidebar_view {
            SidebarView::FileExplorer => self.files.get(self.file_selected).cloned(),
            SidebarView::ModifiedFiles => {
                if self.selected_commit_hash.is_some() {
                    self.modified_files
                        .get(self.modified_selected)
                        .map(|item| item.split_whitespace().next().unwrap_or(item).to_string())
                } else {
                    self.dirty_files
                        .get(self.dirty_selected)
                        .map(|item| item.split_whitespace().next().unwrap_or(item).to_string())
                }
            }
            SidebarView::CommitTimeline | SidebarView::TargetCandidates => {
                if let Some(item) = self.modified_files.get(self.modified_selected) {
                    Some(item.split_whitespace().next().unwrap_or(item).to_string())
                } else {
                    self.files.get(self.file_selected).cloned()
                }
            }
        }
    }

    pub fn perform_timeline_jump(
        &mut self,
        scope: crate::timeline::JumpScope,
        direction: crate::timeline::JumpDirection,
    ) {
        let file_path = match self.current_file_path() {
            Some(path) => path,
            None => {
                self.status_message = "No file selected for timeline jump".to_string();
                return;
            }
        };

        let navigator = crate::timeline::TimelineNavigator::new();
        match navigator.jump(
            &self.repo_path,
            &file_path,
            &self.code_lines,
            self.cursor_line,
            self.selected_commit_hash.as_deref(),
            scope,
            direction,
        ) {
            Ok(Some(result)) => {
                self.update_state_for_commit_hash(result.commit_hash);
                self.active_file = Some(result.file_path);
                self.code_lines = result.code_lines;
                self.code_scroll_offset = 0;
                if self.code_lines.is_empty() {
                    self.cursor_line = 1;
                } else {
                    self.cursor_line = self.cursor_line.clamp(1, self.code_lines.len());
                }
                self.status_message = result.status_message;
                self.update_current_line_blame();
                self.update_file_diff_highlights();
            }
            Ok(None) => {
                self.reset_time_travel();
                self.status_message = format!(
                    "Returned to working directory / latest HEAD version of {}",
                    file_path
                );
            }
            Err(err_msg) => {
                self.status_message = err_msg;
            }
        }
    }

    pub fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        self.code_viewport_height = viewport_height;
        if self.code_lines.is_empty() || viewport_height == 0 {
            self.code_scroll_offset = 0;
            return;
        }

        let total_lines = self.code_lines.len();
        let cursor_idx = self.cursor_line.saturating_sub(1);
        let eff_scrolloff = self.scrolloff.min(viewport_height.saturating_sub(1) / 2);

        let min_top = cursor_idx.saturating_sub(
            viewport_height
                .saturating_sub(1)
                .saturating_sub(eff_scrolloff),
        );
        let max_top = cursor_idx.saturating_sub(eff_scrolloff);

        if self.code_scroll_offset < min_top {
            self.code_scroll_offset = min_top;
        } else if self.code_scroll_offset > max_top {
            self.code_scroll_offset = max_top;
        }

        let max_scroll = total_lines.saturating_sub(viewport_height);
        if self.code_scroll_offset > max_scroll && total_lines >= viewport_height {
            self.code_scroll_offset = max_scroll;
        }
    }

    pub fn scroll_half_page_down(&mut self) {
        match self.active_panel {
            ActivePanel::CodeViewer => {
                let step = (self.code_viewport_height / 2).max(1);
                self.cursor_line = (self.cursor_line + step).min(self.code_lines.len().max(1));
                self.ensure_cursor_visible(self.code_viewport_height);
                self.update_current_line_blame();
                self.update_candidate_commits();
            }
            ActivePanel::Sidebar => {
                let step = (self.sidebar_viewport_height / 2).max(1);
                for _ in 0..step {
                    self.move_selection_down();
                }
            }
        }
    }

    pub fn scroll_half_page_up(&mut self) {
        match self.active_panel {
            ActivePanel::CodeViewer => {
                let step = (self.code_viewport_height / 2).max(1);
                self.cursor_line = self.cursor_line.saturating_sub(step).max(1);
                self.ensure_cursor_visible(self.code_viewport_height);
                self.update_current_line_blame();
                self.update_candidate_commits();
            }
            ActivePanel::Sidebar => {
                let step = (self.sidebar_viewport_height / 2).max(1);
                for _ in 0..step {
                    self.move_selection_up();
                }
            }
        }
    }

    pub fn scroll_page_down(&mut self) {
        match self.active_panel {
            ActivePanel::CodeViewer => {
                let step = self.code_viewport_height.saturating_sub(2).max(1);
                self.cursor_line = (self.cursor_line + step).min(self.code_lines.len().max(1));
                self.ensure_cursor_visible(self.code_viewport_height);
                self.update_current_line_blame();
                self.update_candidate_commits();
            }
            ActivePanel::Sidebar => {
                let step = self.sidebar_viewport_height.saturating_sub(2).max(1);
                for _ in 0..step {
                    self.move_selection_down();
                }
            }
        }
    }

    pub fn scroll_page_up(&mut self) {
        match self.active_panel {
            ActivePanel::CodeViewer => {
                let step = self.code_viewport_height.saturating_sub(2).max(1);
                self.cursor_line = self.cursor_line.saturating_sub(step).max(1);
                self.ensure_cursor_visible(self.code_viewport_height);
                self.update_current_line_blame();
                self.update_candidate_commits();
            }
            ActivePanel::Sidebar => {
                let step = self.sidebar_viewport_height.saturating_sub(2).max(1);
                for _ in 0..step {
                    self.move_selection_up();
                }
            }
        }
    }

    pub fn scroll_line_down(&mut self) {
        if self.active_panel == ActivePanel::CodeViewer && !self.code_lines.is_empty() {
            let max_scroll = self
                .code_lines
                .len()
                .saturating_sub(self.code_viewport_height);
            if self.code_scroll_offset < max_scroll {
                self.code_scroll_offset += 1;
                let eff_scrolloff = self
                    .scrolloff
                    .min(self.code_viewport_height.saturating_sub(1) / 2);
                let min_cursor = self.code_scroll_offset + eff_scrolloff + 1;
                if self.cursor_line < min_cursor {
                    self.cursor_line = min_cursor.min(self.code_lines.len());
                    self.update_current_line_blame();
                    self.update_candidate_commits();
                }
            }
        }
    }

    pub fn scroll_line_up(&mut self) {
        if self.active_panel == ActivePanel::CodeViewer
            && !self.code_lines.is_empty()
            && self.code_scroll_offset > 0
        {
            self.code_scroll_offset -= 1;
            let eff_scrolloff = self
                .scrolloff
                .min(self.code_viewport_height.saturating_sub(1) / 2);
            let max_cursor =
                self.code_scroll_offset + self.code_viewport_height.saturating_sub(eff_scrolloff);
            if self.cursor_line > max_cursor {
                self.cursor_line = max_cursor.max(1);
                self.update_current_line_blame();
                self.update_candidate_commits();
            }
        }
    }

    pub fn center_cursor(&mut self) {
        if self.active_panel == ActivePanel::CodeViewer && !self.code_lines.is_empty() {
            let half_view = self.code_viewport_height / 2;
            self.code_scroll_offset =
                (self.cursor_line.saturating_sub(1)).saturating_sub(half_view);
            let max_scroll = self
                .code_lines
                .len()
                .saturating_sub(self.code_viewport_height);
            if self.code_scroll_offset > max_scroll {
                self.code_scroll_offset = max_scroll;
            }
        }
    }

    pub fn cursor_top(&mut self) {
        if self.active_panel == ActivePanel::CodeViewer && !self.code_lines.is_empty() {
            self.code_scroll_offset = self.cursor_line.saturating_sub(1);
            let max_scroll = self
                .code_lines
                .len()
                .saturating_sub(self.code_viewport_height);
            if self.code_scroll_offset > max_scroll {
                self.code_scroll_offset = max_scroll;
            }
        }
    }

    pub fn cursor_bottom(&mut self) {
        if self.active_panel == ActivePanel::CodeViewer && !self.code_lines.is_empty() {
            let view_minus_1 = self.code_viewport_height.saturating_sub(1);
            self.code_scroll_offset =
                (self.cursor_line.saturating_sub(1)).saturating_sub(view_minus_1);
        }
    }

    pub fn update_current_line_blame(&mut self) {
        let file_path = match self.current_file_path() {
            Some(path) => path,
            None => {
                self.current_line_blame = None;
                return;
            }
        };

        if self.code_lines.is_empty()
            || self.cursor_line == 0
            || self.cursor_line > self.code_lines.len()
        {
            self.current_line_blame = None;
            return;
        }

        if let Ok(repo) = crate::git::GitRepo::open(&self.repo_path) {
            let commit = self.selected_commit_hash.as_deref();
            let blame_res = repo.get_blame_at_commit(
                commit,
                &file_path,
                Some(self.cursor_line),
                Some(self.cursor_line),
            );
            if let Ok(blame_lines) = blame_res {
                self.current_line_blame = blame_lines.into_iter().next();
            } else {
                self.current_line_blame = None;
            }
        } else {
            self.current_line_blame = None;
        }
    }

    pub fn active_scope(&self) -> Scope {
        match self.active_panel {
            ActivePanel::Sidebar => Scope::Sidebar,
            ActivePanel::CodeViewer => Scope::CodeViewer,
        }
    }

    pub fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                if self.selected_commit_hash.is_some() {
                    self.reset_time_travel();
                } else {
                    self.quit();
                }
            }
            Action::ToggleFocus => self.toggle_panel_focus(),
            Action::SetSidebarView(index) => match index {
                1 => self.set_sidebar_view(SidebarView::FileExplorer),
                2 => self.set_sidebar_view(SidebarView::ModifiedFiles),
                3 => self.set_sidebar_view(SidebarView::CommitTimeline),
                4 => self.set_sidebar_view(SidebarView::TargetCandidates),
                _ => {}
            },
            Action::CycleNavMode => self.cycle_navigation_mode(),
            Action::MoveUp => self.move_selection_up(),
            Action::MoveDown => self.move_selection_down(),
            Action::HalfPageDown => self.scroll_half_page_down(),
            Action::HalfPageUp => self.scroll_half_page_up(),
            Action::PageDown => self.scroll_page_down(),
            Action::PageUp => self.scroll_page_up(),
            Action::ScrollLineDown => self.scroll_line_down(),
            Action::ScrollLineUp => self.scroll_line_up(),
            Action::CenterCursor => self.center_cursor(),
            Action::CursorTop => self.cursor_top(),
            Action::CursorBottom => self.cursor_bottom(),
            Action::Select => match self.sidebar_view {
                SidebarView::FileExplorer | SidebarView::ModifiedFiles => {
                    self.load_currently_selected_file();
                }
                SidebarView::CommitTimeline => {
                    if let Some((hash, msg)) = self.commits.get(self.commit_selected).cloned() {
                        self.selected_commit_hash = Some(hash.clone());
                        self.update_modified_files_for_selected_commit();
                        self.sidebar_view = SidebarView::ModifiedFiles;
                        self.load_currently_selected_file();
                        self.status_message =
                            format!("Viewing modified files for commit {} ({})", hash, msg);
                    } else {
                        self.status_message = "No commit selected".to_string();
                    }
                }
                SidebarView::TargetCandidates => {
                    if let Some((hash, msg)) =
                        self.candidate_commits.get(self.candidate_selected).cloned()
                    {
                        self.selected_commit_hash = Some(hash.clone());
                        self.update_modified_files_for_selected_commit();
                        self.sidebar_view = SidebarView::ModifiedFiles;
                        self.load_currently_selected_file();
                        self.status_message =
                            format!("Viewing candidate commit {} ({})", hash, msg);
                    } else {
                        self.status_message = "No candidate commit selected".to_string();
                    }
                }
            },
            Action::JumpNextAuto => {
                let scope = match self.nav_mode {
                    NavigationMode::Commit => crate::timeline::JumpScope::Commit,
                    NavigationMode::File => crate::timeline::JumpScope::File,
                    NavigationMode::Function => crate::timeline::JumpScope::Function,
                    NavigationMode::Line => crate::timeline::JumpScope::Line,
                };
                self.perform_timeline_jump(scope, crate::timeline::JumpDirection::Next);
            }
            Action::JumpPrevAuto => {
                let scope = match self.nav_mode {
                    NavigationMode::Commit => crate::timeline::JumpScope::Commit,
                    NavigationMode::File => crate::timeline::JumpScope::File,
                    NavigationMode::Function => crate::timeline::JumpScope::Function,
                    NavigationMode::Line => crate::timeline::JumpScope::Line,
                };
                self.perform_timeline_jump(scope, crate::timeline::JumpDirection::Previous);
            }
            Action::JumpNextCommit => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Commit,
                    crate::timeline::JumpDirection::Next,
                );
            }
            Action::JumpPrevCommit => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Commit,
                    crate::timeline::JumpDirection::Previous,
                );
            }
            Action::JumpNextFile => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::File,
                    crate::timeline::JumpDirection::Next,
                );
            }
            Action::JumpPrevFile => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::File,
                    crate::timeline::JumpDirection::Previous,
                );
            }
            Action::JumpNextFunction => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Function,
                    crate::timeline::JumpDirection::Next,
                );
            }
            Action::JumpPrevFunction => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Function,
                    crate::timeline::JumpDirection::Previous,
                );
            }
            Action::JumpNextLine => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Line,
                    crate::timeline::JumpDirection::Next,
                );
            }
            Action::JumpPrevLine => {
                self.perform_timeline_jump(
                    crate::timeline::JumpScope::Line,
                    crate::timeline::JumpDirection::Previous,
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
        assert_eq!(app.nav_mode, NavigationMode::Commit);
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
        assert_eq!(app.nav_mode, NavigationMode::Commit);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::File);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::Function);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::Line);

        app.cycle_navigation_mode();
        assert_eq!(app.nav_mode, NavigationMode::Commit);
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
