use std::path::PathBuf;

pub mod file_tree;
pub use file_tree::*;

mod types;
pub use types::*;

use crate::git::{BlameLine, GitRepo};
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
    pub repo: Option<GitRepo>,
    pub active_panel: ActivePanel,
    pub sidebar_view: SidebarView,
    pub timeline_filter: TimelineFilter,
    pub nav_mode: NavigationMode,
    pub active_file: Option<String>,

    pub files: Vec<String>,
    pub file_selected: usize,
    pub expanded_folders: std::collections::HashSet<String>,

    pub dirty_files: Vec<ModifiedFileEntry>,
    pub dirty_selected: usize,

    pub modified_files: Vec<ModifiedFileEntry>,
    pub modified_selected: usize,

    pub uncommitted_files: Vec<ModifiedFileEntry>,

    pub commits: Vec<CommitSummary>,
    pub commit_selected: usize,
    pub candidate_commits: Vec<CommitSummary>,
    pub candidate_selected: usize,
    pub selected_commit_hash: Option<String>,

    pub code_lines: Vec<String>,
    pub cursor_line: usize,        // 1-based index
    pub code_scroll_offset: usize, // 0-based top visible line index
    pub code_viewport_height: usize,
    pub sidebar_viewport_height: usize,
    pub scrolloff: usize,

    pub status_message: String,
    pub exit_message: Option<String>,
    pub running: bool,
    pub in_alternate_screen: bool,
    pub show_help: bool,
    pub show_splashscreen: bool,
    pub git_version: Option<String>,

    pub theme_bg: Option<ratatui::style::Color>,
    pub theme_fg: Option<ratatui::style::Color>,

    pub grammar_registry: GrammarRegistry,
    pub current_line_blame: Option<BlameLine>,
    pub file_diff_highlights: std::collections::HashMap<usize, crate::git::DiffLineType>,
    pub blame_cache: std::collections::HashMap<(String, Option<String>), Vec<BlameLine>>,
    pub blame_subprocess_count: usize,
    pub last_loaded_file: Option<String>,
    pub last_loaded_commit: Option<String>,
    pub file_view_mode: FileViewMode,
    pub last_loaded_view_mode: Option<FileViewMode>,
    pub render_markdown_formatted: bool,

    pub input_prompt: Option<InputPrompt>,
    pub input_buffer: String,
    pub last_search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub search_match_index: usize,
    pub symbol_matches: Vec<crate::treesitter::SymbolItem>,
    pub symbol_selected: usize,
}

impl AppState {
    pub fn new(repo_path: PathBuf) -> Self {
        let repo = GitRepo::open(&repo_path).ok();
        let git_version = std::process::Command::new("git")
            .arg("--version")
            .output()
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|s| s.trim().to_string());

        Self {
            repo_path,
            repo,
            git_version,
            active_panel: ActivePanel::Sidebar,
            sidebar_view: SidebarView::FileExplorer,
            timeline_filter: TimelineFilter::All,
            nav_mode: NavigationMode::Commit,
            active_file: None,

            files: Vec::new(),
            file_selected: 0,
            expanded_folders: std::collections::HashSet::new(),

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

            status_message:
                "Press 'Tab' or 'h'/'l' to switch focus. '?' for help. 'm' for nav mode."
                    .to_string(),
            exit_message: None,
            running: true,
            in_alternate_screen: false,
            show_help: false,
            show_splashscreen: false,

            theme_bg: None,
            theme_fg: None,

            grammar_registry: GrammarRegistry::new(),
            current_line_blame: None,
            file_diff_highlights: std::collections::HashMap::new(),
            blame_cache: std::collections::HashMap::new(),
            blame_subprocess_count: 0,
            last_loaded_file: None,
            last_loaded_commit: None,
            file_view_mode: FileViewMode::Full,
            last_loaded_view_mode: None,
            render_markdown_formatted: true,

            input_prompt: None,
            input_buffer: String::new(),
            last_search_query: None,
            search_matches: Vec::new(),
            search_match_index: 0,
            symbol_matches: Vec::new(),
            symbol_selected: 0,
        }
    }

    pub fn visible_file_items(&self) -> Vec<VisibleFileItem> {
        let tree = build_file_tree(&self.files);
        flatten_file_tree(&tree, &self.expanded_folders)
    }

    pub fn active_modified_files(&self) -> &[ModifiedFileEntry] {
        if self.selected_commit_hash.is_some() {
            &self.modified_files
        } else {
            &self.dirty_files
        }
    }

    pub fn file_modified_status(&self, path: &str) -> Option<&str> {
        let clean_target = path.trim_start_matches("./");
        let entries = self.active_modified_files();
        entries.iter().find_map(|e| {
            let clean_entry = e.path.trim_start_matches("./");
            if clean_entry == clean_target {
                let status = e.status.trim();
                if status.is_empty() {
                    Some("M")
                } else {
                    Some(status)
                }
            } else {
                None
            }
        })
    }

    pub fn dir_has_modified_files(&self, dir_path: &str) -> bool {
        let clean_dir = dir_path.trim_start_matches("./");
        if clean_dir.is_empty() {
            return false;
        }
        let prefix = if clean_dir.ends_with('/') {
            clean_dir.to_string()
        } else {
            format!("{}/", clean_dir)
        };

        let entries = self.active_modified_files();
        entries.iter().any(|e| {
            let clean_entry = e.path.trim_start_matches("./");
            clean_entry.starts_with(&prefix)
        })
    }

    pub fn expand_all_folders(&mut self) {
        let tree = build_file_tree(&self.files);
        collect_all_dir_paths(&tree, &mut self.expanded_folders);
    }

    pub fn expand_folder(&mut self, path: &str) {
        self.expanded_folders.insert(path.to_string());
    }

    pub fn collapse_folder(&mut self, path: &str) {
        self.expanded_folders.remove(path);
    }

    pub fn toggle_folder(&mut self, path: &str) {
        if self.expanded_folders.contains(path) {
            self.expanded_folders.remove(path);
        } else {
            self.expanded_folders.insert(path.to_string());
        }
    }

    pub fn move_to_parent_folder(&mut self) {
        let items = self.visible_file_items();
        if items.is_empty() || self.file_selected >= items.len() {
            return;
        }

        let current_item = &items[self.file_selected];
        let parent_path = if let Some(slash_idx) = current_item.path.rfind('/') {
            &current_item.path[..slash_idx]
        } else {
            ""
        };

        if !parent_path.is_empty() {
            if let Some(parent_idx) = items
                .iter()
                .position(|it| it.path == parent_path && it.is_dir)
            {
                self.file_selected = parent_idx;
                self.load_currently_selected_file();
            }
        }
    }

    pub fn ensure_repo(&mut self) -> Option<&GitRepo> {
        if self.repo.is_none() {
            self.repo = GitRepo::open(&self.repo_path).ok();
        }
        self.repo.as_ref()
    }

    pub fn repo(&self) -> Option<GitRepo> {
        if let Some(ref r) = self.repo {
            Some(r.clone())
        } else {
            GitRepo::open(&self.repo_path).ok()
        }
    }

    pub fn repo_ref(&self) -> Option<&GitRepo> {
        self.repo.as_ref()
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
        if view == SidebarView::CommitTimeline {
            self.update_candidate_commits();
        }
        self.load_currently_selected_file();
        self.status_message = format!("Sidebar view: {}", view.name());
    }

    pub fn toggle_timeline_filter(&mut self) {
        self.timeline_filter = self.timeline_filter.toggle();
        if self.timeline_filter == TimelineFilter::Candidates {
            self.update_candidate_commits();
        }
        self.load_currently_selected_file();
        self.status_message = format!("Timeline filter: {}", self.timeline_filter.name());
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
                    let items = self.visible_file_items();
                    if !items.is_empty() && self.file_selected + 1 < items.len() {
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
                    if self.timeline_filter == TimelineFilter::All {
                        if !self.commits.is_empty() && self.commit_selected + 1 < self.commits.len()
                        {
                            self.commit_selected += 1;
                            self.update_modified_files_for_selected_commit();
                            self.load_currently_selected_file();
                        }
                    } else if !self.candidate_commits.is_empty()
                        && self.candidate_selected + 1 < self.candidate_commits.len()
                    {
                        self.candidate_selected += 1;
                        let hash = self.candidate_commits[self.candidate_selected].hash.clone();
                        self.update_state_for_commit_hash(hash);
                        self.load_currently_selected_file();
                    }
                }
            },
            ActivePanel::CodeViewer => {
                if !self.code_lines.is_empty() && self.cursor_line < self.code_lines.len() {
                    self.cursor_line += 1;
                    self.ensure_cursor_visible(self.code_viewport_height);
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
                    if self.timeline_filter == TimelineFilter::All {
                        if self.commit_selected > 0 {
                            self.commit_selected -= 1;
                            self.update_modified_files_for_selected_commit();
                            self.load_currently_selected_file();
                        }
                    } else if self.candidate_selected > 0 {
                        self.candidate_selected -= 1;
                        let hash = self.candidate_commits[self.candidate_selected].hash.clone();
                        self.update_state_for_commit_hash(hash);
                        self.load_currently_selected_file();
                    }
                }
            },
            ActivePanel::CodeViewer => {
                if self.cursor_line > 1 {
                    self.cursor_line -= 1;
                    self.ensure_cursor_visible(self.code_viewport_height);
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
        self.ensure_repo();
        let repo = match self.repo() {
            Some(r) => r,
            None => return,
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
                    let effective_line = self.effective_cursor_line();
                    let range = crate::treesitter::scope::find_enclosing_function_range(
                        &self.grammar_registry,
                        f,
                        &source_code,
                        effective_line,
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
                    let effective_line = self.effective_cursor_line();
                    repo.get_line_commits(f, effective_line, effective_line, None)
                } else {
                    repo.get_commit_history(None)
                }
            }
        };

        if let Ok(commits) = commits_res {
            self.candidate_commits = commits.into_iter().map(CommitSummary::from).collect();

            if let Some(hash) = &self.selected_commit_hash {
                if let Some(idx) = self
                    .candidate_commits
                    .iter()
                    .position(|c| c.matches_hash(hash))
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

        let hash = self.commits[self.commit_selected].hash.clone();
        self.update_state_for_commit_hash(hash);
    }

    pub fn update_state_for_commit_hash(&mut self, hash: String) {
        self.ensure_repo();
        self.selected_commit_hash = Some(hash.clone());

        // Sync commit_selected index in self.commits if hash exists in commit history
        if let Some(idx) = self.commits.iter().position(|c| c.matches_hash(&hash)) {
            self.commit_selected = idx;
        }

        if let Some(idx) = self
            .candidate_commits
            .iter()
            .position(|c| c.matches_hash(&hash))
        {
            self.candidate_selected = idx;
        }

        // Fetch modified files for this commit
        if let Some(repo) = self.repo() {
            if let Ok(commit_files) = repo.get_commit_files(&hash) {
                self.modified_files = commit_files
                    .into_iter()
                    .map(ModifiedFileEntry::from)
                    .collect();

                // If current file is in modified_files, set modified_selected to match it
                if let Some(cur_file) = self.current_file_path() {
                    if let Some(f_idx) = self
                        .modified_files
                        .iter()
                        .position(|item| item.path == cur_file)
                    {
                        self.modified_selected = f_idx;
                    } else {
                        self.modified_selected = 0;
                    }
                } else {
                    self.modified_selected = 0;
                }
            }

            // Update file explorer list for this target commit
            if let Ok(tree_files) = repo.list_files_at_commit(&hash) {
                self.files = tree_files;
                if let Some(cur_file) = self.current_file_path() {
                    let items = self.visible_file_items();
                    if let Some(f_idx) = items.iter().position(|it| it.path == cur_file) {
                        self.file_selected = f_idx;
                    } else if !items.is_empty() {
                        self.file_selected = self.file_selected.min(items.len() - 1);
                    } else {
                        self.file_selected = 0;
                    }
                } else {
                    self.file_selected = 0;
                }
            }
        }

        self.update_file_diff_highlights();
    }

    pub fn reset_time_travel(&mut self) {
        self.ensure_repo();
        self.selected_commit_hash = None;
        if let Some(repo) = self.repo() {
            if let Ok(wd_files) = repo.list_files() {
                self.files = wd_files;
                if let Some(cur_file) = self.current_file_path() {
                    let items = self.visible_file_items();
                    if let Some(f_idx) = items.iter().position(|it| it.path == cur_file) {
                        self.file_selected = f_idx;
                    } else if !items.is_empty() {
                        self.file_selected = self.file_selected.min(items.len() - 1);
                    } else {
                        self.file_selected = 0;
                    }
                } else {
                    self.file_selected = 0;
                }
            }
        }
        self.clear_blame_cache();
        self.load_currently_selected_file();
        self.status_message = "Exited Time-Travel mode (returned to working directory)".to_string();
    }

    pub fn clear_blame_cache(&mut self) {
        self.blame_cache.clear();
    }

    pub fn reload_repo_data(&mut self) {
        if let Some(repo) = self.repo() {
            if let Ok(files) = repo.list_files() {
                self.files = files;
                self.expand_all_folders();
            }
            if let Ok(statuses) = repo.get_status() {
                let items: Vec<ModifiedFileEntry> =
                    statuses.into_iter().map(ModifiedFileEntry::from).collect();
                self.uncommitted_files = items.clone();
                self.dirty_files = items;
            }
            if let Ok(commits) = repo.get_commit_history(Some(50)) {
                self.commits = commits.into_iter().map(CommitSummary::from).collect();
            }
            self.load_currently_selected_file();
        }
    }

    pub fn trigger_edit_here(&mut self) -> crate::rebase::RebaseResult {
        let target_hash = match if self.sidebar_view == SidebarView::CommitTimeline {
            if self.timeline_filter == TimelineFilter::All {
                self.commits
                    .get(self.commit_selected)
                    .map(|c| c.hash.clone())
            } else {
                self.candidate_commits
                    .get(self.candidate_selected)
                    .map(|c| c.hash.clone())
            }
        } else {
            self.selected_commit_hash.clone().or_else(|| {
                self.commits
                    .get(self.commit_selected)
                    .map(|c| c.hash.clone())
            })
        } {
            Some(h) => h,
            None => {
                self.status_message = "No commit selected for Edit Here".to_string();
                return crate::rebase::RebaseResult::Error("No commit selected".to_string());
            }
        };

        let stdin = std::io::stdin();
        let result = crate::rebase::execute_edit_here(&self.repo_path, &target_hash, stdin.lock());

        match &result {
            crate::rebase::RebaseResult::Completed => {
                self.in_alternate_screen = true;
                let short_hash = &target_hash[..7.min(target_hash.len())];
                self.status_message = format!("Successfully edited commit {}", short_hash);
                self.reload_repo_data();
            }
            crate::rebase::RebaseResult::Aborted => {
                self.in_alternate_screen = true;
                self.status_message = "Rebase aborted. Restored repository state.".to_string();
                self.reload_repo_data();
            }
            crate::rebase::RebaseResult::ConflictExited(opt_msg) => {
                self.in_alternate_screen = false;
                self.exit_message = opt_msg.clone();
                self.running = false;
            }
            crate::rebase::RebaseResult::Error(msg) => {
                self.status_message = format!("Edit Here error: {}", msg);
            }
        }

        result
    }

    pub fn load_currently_selected_file(&mut self) {
        self.ensure_repo();
        match self.sidebar_view {
            SidebarView::FileExplorer => {
                let items = self.visible_file_items();
                if !items.is_empty() {
                    self.file_selected = self.file_selected.min(items.len() - 1);
                    let item = &items[self.file_selected];
                    if !item.is_dir || self.file_view_mode == FileViewMode::Diff {
                        self.active_file = Some(item.path.clone());
                    } else {
                        self.active_file = None;
                        self.code_lines.clear();
                        self.cursor_line = 1;
                        self.code_scroll_offset = 0;
                        self.last_loaded_file = None;
                        self.last_loaded_commit = None;
                        self.last_loaded_view_mode = None;
                        self.status_message = format!("Selected directory: {}", item.name);
                        return;
                    }
                }
            }
            SidebarView::ModifiedFiles => {
                if self.selected_commit_hash.is_some() {
                    if let Some(item) = self.modified_files.get(self.modified_selected) {
                        self.active_file = Some(item.path.clone());
                    }
                } else if let Some(item) = self.dirty_files.get(self.dirty_selected) {
                    self.active_file = Some(item.path.clone());
                }
            }
            SidebarView::CommitTimeline => {
                if self.active_file.is_none() {
                    if let Some(item) = self.modified_files.get(self.modified_selected) {
                        self.active_file = Some(item.path.clone());
                    } else if let Some(f) = self.files.get(self.file_selected) {
                        self.active_file = Some(f.clone());
                    }
                }
            }
        }

        let target_file = self.current_file_path();
        let target_commit = self.selected_commit_hash.clone();
        let target_view_mode = self.file_view_mode;

        let clean_path = target_file;

        let old_cursor_line = self.cursor_line;
        let old_scroll_offset = self.code_scroll_offset;
        let visual_row = old_cursor_line.saturating_sub(old_scroll_offset + 1);

        let mut loaded_lines: Option<Vec<String>> = None;

        if target_view_mode == FileViewMode::Diff {
            if let Some(repo) = self.repo() {
                let diff_res = if let Some(hash) = &target_commit {
                    if let Some(path) = &clean_path {
                        repo.get_diff_file(hash, path)
                    } else {
                        repo.get_diff_commit(hash)
                    }
                } else {
                    repo.get_working_diff(clean_path.as_deref())
                };

                match diff_res {
                    Ok(diff_text) => {
                        let trimmed = diff_text.trim();
                        if trimmed.is_empty() {
                            loaded_lines =
                                Some(vec!["(No diff for working directory / file)".to_string()]);
                        } else {
                            loaded_lines = Some(diff_text.lines().map(|s| s.to_string()).collect());
                        }
                        if let Some(hash) = &target_commit {
                            let short_hash = &hash[..7.min(hash.len())];
                            if let Some(path) = &clean_path {
                                self.status_message =
                                    format!("Loaded diff for {} at commit {}", path, short_hash);
                            } else {
                                self.status_message =
                                    format!("Loaded diff for commit {}", short_hash);
                            }
                        } else if let Some(path) = &clean_path {
                            self.status_message = format!("Loaded working diff for {}", path);
                        } else {
                            self.status_message = "Loaded working directory diff".to_string();
                        }
                    }
                    Err(_) => {
                        let msg = clean_path
                            .as_deref()
                            .map(|p| format!("Could not load diff for {}", p))
                            .unwrap_or_else(|| "Could not load working directory diff".to_string());
                        loaded_lines = Some(vec![msg.clone()]);
                        self.status_message = msg;
                    }
                }
            } else {
                loaded_lines = Some(vec!["Not a git repository".to_string()]);
            }
        } else {
            let file_path_str = match clean_path.clone() {
                Some(p) => p,
                None => {
                    self.code_lines.clear();
                    self.cursor_line = 1;
                    self.code_scroll_offset = 0;
                    self.last_loaded_file = None;
                    self.last_loaded_commit = None;
                    self.last_loaded_view_mode = None;
                    self.update_current_line_blame();
                    self.update_candidate_commits();
                    self.update_file_diff_highlights();
                    return;
                }
            };

            if let Some(hash) = &target_commit {
                let short_hash = &hash[..7.min(hash.len())];
                if let Some(repo) = self.repo() {
                    match repo.get_file_at_commit(hash, &file_path_str) {
                        Ok(content) => {
                            loaded_lines = Some(content.lines().map(|s| s.to_string()).collect());
                            self.status_message =
                                format!("Loaded {} at commit {}", file_path_str, short_hash);
                        }
                        Err(_) => {
                            loaded_lines = Some(vec![format!(
                                "File '{}' did not exist at commit {}",
                                file_path_str, short_hash
                            )]);
                            self.status_message = format!(
                                "File '{}' did not exist at commit {}",
                                file_path_str, short_hash
                            );
                        }
                    }
                }
            } else {
                let file_path = self.repo_path.join(&file_path_str);
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    loaded_lines = Some(content.lines().map(|s| s.to_string()).collect());
                    self.status_message = format!("Loaded file: {}", file_path_str);
                } else {
                    loaded_lines = Some(Vec::new());
                    self.status_message = format!("Could not read file: {}", file_path_str);
                }
            }
        }

        if let Some(new_lines) = loaded_lines {
            let same_file = self.last_loaded_file == clean_path;
            let same_commit = self.last_loaded_commit == target_commit;
            let same_view_mode = self.last_loaded_view_mode == Some(target_view_mode);

            if same_file && same_view_mode && !new_lines.is_empty() {
                if same_commit {
                    let new_cursor = old_cursor_line.clamp(1, new_lines.len());
                    let max_scroll = old_scroll_offset.min(new_lines.len().saturating_sub(1));

                    self.code_lines = new_lines;
                    self.cursor_line = new_cursor;
                    self.code_scroll_offset = max_scroll;
                } else if let Some(path) = &clean_path {
                    let diff_text = self
                        .repo()
                        .and_then(|repo| {
                            repo.get_diff_between(
                                self.last_loaded_commit.as_deref(),
                                target_commit.as_deref(),
                                path,
                            )
                            .ok()
                        })
                        .unwrap_or_default();

                    let mapped_line =
                        crate::git::diff_parser::map_line_number(&diff_text, old_cursor_line);
                    let new_cursor = mapped_line.clamp(1, new_lines.len());
                    let new_scroll = new_cursor.saturating_sub(visual_row + 1);
                    let max_scroll = new_lines.len().saturating_sub(1);
                    let clamped_scroll = new_scroll.min(max_scroll);

                    self.code_lines = new_lines;
                    self.cursor_line = new_cursor;
                    self.code_scroll_offset = clamped_scroll;
                } else {
                    self.code_lines = new_lines;
                    self.cursor_line = 1;
                    self.code_scroll_offset = 0;
                }

                if self.code_viewport_height > 0 {
                    self.ensure_cursor_visible(self.code_viewport_height);
                }
            } else {
                self.code_lines = new_lines;
                self.cursor_line = 1;
                self.code_scroll_offset = 0;
            }
        }

        self.last_loaded_file = clean_path;
        self.last_loaded_commit = target_commit;
        self.last_loaded_view_mode = Some(target_view_mode);

        self.update_current_line_blame();
        self.update_candidate_commits();
        self.update_file_diff_highlights();
    }

    pub fn toggle_file_view_mode(&mut self) {
        self.file_view_mode = self.file_view_mode.toggle();
        self.status_message = format!("File view mode: {}", self.file_view_mode.name());
        self.load_currently_selected_file();
    }

    pub fn update_file_diff_highlights(&mut self) {
        self.ensure_repo();
        self.file_diff_highlights.clear();
        if self.file_view_mode == FileViewMode::Diff {
            return;
        }
        let cur_file = match self.current_file_path() {
            Some(f) => f,
            None => return,
        };

        let diff_res = if let Some(repo) = self.repo() {
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
            SidebarView::FileExplorer => {
                let items = self.visible_file_items();
                if let Some(item) = items.get(self.file_selected) {
                    if !item.is_dir || self.file_view_mode == FileViewMode::Diff {
                        Some(item.path.clone())
                    } else {
                        self.files.first().cloned()
                    }
                } else {
                    self.files.first().cloned()
                }
            }
            SidebarView::ModifiedFiles => {
                if self.selected_commit_hash.is_some() {
                    self.modified_files
                        .get(self.modified_selected)
                        .map(|item| item.path.clone())
                } else {
                    self.dirty_files
                        .get(self.dirty_selected)
                        .map(|item| item.path.clone())
                }
            }
            SidebarView::CommitTimeline => {
                if let Some(item) = self.modified_files.get(self.modified_selected) {
                    Some(item.path.clone())
                } else {
                    self.files.get(self.file_selected).cloned()
                }
            }
        }
    }

    pub fn effective_cursor_line(&self) -> usize {
        crate::git::diff_parser::resolve_file_line_number(&self.code_lines, self.cursor_line)
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

        let effective_line = self.effective_cursor_line();
        let navigator = crate::timeline::TimelineNavigator::new();
        self.ensure_repo();
        let repo_opt = self.repo();
        match navigator.jump(crate::timeline::TimelineJumpRequest {
            repo_path: &self.repo_path,
            repo: repo_opt.as_ref(),
            file_path: &file_path,
            source_lines: &self.code_lines,
            cursor_line: effective_line,
            current_commit_hash: self.selected_commit_hash.as_deref(),
            scope,
            direction,
        }) {
            Ok(Some(result)) => {
                let status_msg = result.status_message;
                self.update_state_for_commit_hash(result.commit_hash);
                self.active_file = Some(result.file_path);
                self.load_currently_selected_file();
                self.status_message = status_msg;
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
        self.ensure_repo();
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

        let key = (file_path.clone(), self.selected_commit_hash.clone());

        if !self.blame_cache.contains_key(&key) {
            if let Some(repo) = self.repo() {
                let commit = self.selected_commit_hash.as_deref();
                self.blame_subprocess_count += 1;
                let blame_res = repo.get_blame_at_commit(commit, &file_path, None, None);
                let blame_lines = blame_res.unwrap_or_default();
                self.blame_cache.insert(key.clone(), blame_lines);
            } else {
                self.blame_cache.insert(key.clone(), Vec::new());
            }
        }

        if let Some(blame_lines) = self.blame_cache.get(&key) {
            let effective_line = self.effective_cursor_line();
            if effective_line > 0 {
                let found = blame_lines
                    .get(effective_line - 1)
                    .filter(|b| b.final_line == effective_line)
                    .or_else(|| blame_lines.iter().find(|b| b.final_line == effective_line));
                self.current_line_blame = found.cloned();
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
                if self.show_help {
                    self.show_help = false;
                    self.status_message = "Closed keybindings help overlay.".to_string();
                } else if self.show_splashscreen {
                    self.show_splashscreen = false;
                    self.status_message = "Closed TARDIS splashscreen.".to_string();
                } else if self.selected_commit_hash.is_some() {
                    self.reset_time_travel();
                } else {
                    self.quit();
                }
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.status_message = "Opened keybindings help overlay.".to_string();
                } else {
                    self.status_message = "Closed keybindings help overlay.".to_string();
                }
            }
            Action::ToggleSplashscreen => {
                self.show_splashscreen = !self.show_splashscreen;
                if self.show_splashscreen {
                    self.status_message = "Opened TARDIS splashscreen.".to_string();
                } else {
                    self.status_message = "Closed TARDIS splashscreen.".to_string();
                }
            }
            Action::ToggleMarkdownFormat => {
                self.render_markdown_formatted = !self.render_markdown_formatted;
                self.status_message = format!(
                    "Markdown formatted rendering: {}",
                    if self.render_markdown_formatted {
                        "ENABLED"
                    } else {
                        "DISABLED"
                    }
                );
            }
            Action::ToggleFileViewMode => {
                self.toggle_file_view_mode();
            }
            Action::ToggleTimelineFilter => {
                self.toggle_timeline_filter();
            }
            Action::PromptGotoLine => {
                self.input_prompt = Some(InputPrompt::GotoLine);
                self.input_buffer.clear();
                self.status_message = "Go to line:".to_string();
            }
            Action::PromptSearchText => {
                self.input_prompt = Some(InputPrompt::SearchText);
                self.input_buffer.clear();
                self.status_message = "Search text:".to_string();
            }
            Action::PromptSearchSymbol => {
                self.open_symbol_prompt();
            }
            Action::SearchNext => {
                self.search_next();
            }
            Action::SearchPrev => {
                self.search_prev();
            }
            Action::ToggleFocus => self.toggle_panel_focus(),
            Action::SetSidebarView(index) => match index {
                1 => self.set_sidebar_view(SidebarView::FileExplorer),
                2 => self.set_sidebar_view(SidebarView::ModifiedFiles),
                3 => self.set_sidebar_view(SidebarView::CommitTimeline),
                4 => {
                    self.set_sidebar_view(SidebarView::CommitTimeline);
                    self.timeline_filter = TimelineFilter::Candidates;
                    self.update_candidate_commits();
                    self.load_currently_selected_file();
                }
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
                SidebarView::FileExplorer => {
                    let items = self.visible_file_items();
                    if let Some(item) = items.get(self.file_selected).cloned() {
                        if item.is_dir {
                            self.toggle_folder(&item.path);
                        } else {
                            self.load_currently_selected_file();
                        }
                    }
                }
                SidebarView::ModifiedFiles => {
                    self.load_currently_selected_file();
                }
                SidebarView::CommitTimeline => {
                    let selected_commit = if self.timeline_filter == TimelineFilter::All {
                        self.commits.get(self.commit_selected).cloned()
                    } else {
                        self.candidate_commits.get(self.candidate_selected).cloned()
                    };

                    if let Some(commit) = selected_commit {
                        self.selected_commit_hash = Some(commit.hash.clone());
                        self.update_modified_files_for_selected_commit();
                        self.sidebar_view = SidebarView::ModifiedFiles;
                        self.load_currently_selected_file();
                        self.status_message = format!(
                            "Viewing modified files for commit {} ({})",
                            commit.short_hash, commit.message
                        );
                    } else {
                        self.status_message = "No commit selected".to_string();
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
                self.trigger_edit_here();
            }
            Action::ExpandFolder => {
                if self.sidebar_view == SidebarView::FileExplorer {
                    let items = self.visible_file_items();
                    if let Some(item) = items.get(self.file_selected).cloned() {
                        if item.is_dir {
                            if !item.is_expanded {
                                self.expand_folder(&item.path);
                            } else if self.file_selected + 1 < items.len() {
                                self.file_selected += 1;
                                self.load_currently_selected_file();
                            }
                        } else {
                            self.load_currently_selected_file();
                        }
                    }
                }
            }
            Action::CollapseFolder => {
                if self.sidebar_view == SidebarView::FileExplorer {
                    let items = self.visible_file_items();
                    if let Some(item) = items.get(self.file_selected).cloned() {
                        if item.is_dir && item.is_expanded {
                            self.collapse_folder(&item.path);
                        } else {
                            self.move_to_parent_folder();
                        }
                    }
                }
            }
            Action::ToggleFolder => {
                if self.sidebar_view == SidebarView::FileExplorer {
                    let items = self.visible_file_items();
                    if let Some(item) = items.get(self.file_selected).cloned() {
                        if item.is_dir {
                            self.toggle_folder(&item.path);
                        } else {
                            self.load_currently_selected_file();
                        }
                    }
                }
            }
        }
    }

    pub fn goto_line(&mut self, line: usize) {
        if self.code_lines.is_empty() {
            return;
        }
        let target = line.clamp(1, self.code_lines.len());
        self.cursor_line = target;
        if self.code_viewport_height > 0 {
            self.ensure_cursor_visible(self.code_viewport_height);
        }
        self.update_current_line_blame();
        self.update_candidate_commits();
        self.status_message = format!("Jumped to line {}", target);
    }

    pub fn open_file_at_line<P: AsRef<std::path::Path>>(
        &mut self,
        file_path: P,
        line: Option<usize>,
    ) {
        let raw_path = file_path.as_ref();

        // Determine repository/worktree top-level root
        let toplevel = self
            .repo()
            .and_then(|r| r.get_toplevel().ok())
            .or_else(|| std::fs::canonicalize(&self.repo_path).ok())
            .unwrap_or_else(|| self.repo_path.clone());

        let canonical_toplevel = std::fs::canonicalize(&toplevel).unwrap_or(toplevel);

        // Resolve absolute target file path
        let abs_file = if raw_path.is_absolute() {
            std::fs::canonicalize(raw_path).unwrap_or_else(|_| raw_path.to_path_buf())
        } else {
            let joined = self.repo_path.join(raw_path);
            std::fs::canonicalize(&joined).unwrap_or(joined)
        };

        // Strip repo/worktree top-level prefix to obtain repo-relative path
        let clean_path = if let Ok(rel) = abs_file.strip_prefix(&canonical_toplevel) {
            rel.to_string_lossy().to_string()
        } else {
            raw_path
                .to_string_lossy()
                .trim_start_matches("./")
                .to_string()
        };

        // Automatically expand parent folders of target file so it is visible in the file tree
        let parts: Vec<&str> = clean_path.split('/').collect();
        let mut current_dir = String::new();
        for part in parts.iter().take(parts.len().saturating_sub(1)) {
            if !current_dir.is_empty() {
                current_dir.push('/');
            }
            current_dir.push_str(part);
            self.expanded_folders.insert(current_dir.clone());
        }

        self.active_file = Some(clean_path.clone());

        let items = self.visible_file_items();
        if let Some(idx) = items
            .iter()
            .position(|it| it.path.trim_start_matches("./") == clean_path)
        {
            self.file_selected = idx;
        }

        self.load_currently_selected_file();
        self.active_panel = ActivePanel::CodeViewer;

        if let Some(line_num) = line {
            self.goto_line(line_num);
        }
    }

    pub fn set_theme_colors(
        &mut self,
        bg: Option<ratatui::style::Color>,
        fg: Option<ratatui::style::Color>,
    ) {
        self.theme_bg = bg;
        self.theme_fg = fg;
    }

    pub fn search_text(&mut self, query: &str) {
        if query.is_empty() || self.code_lines.is_empty() {
            return;
        }
        self.last_search_query = Some(query.to_string());
        self.search_matches.clear();
        let lower_query = query.to_lowercase();

        for (idx, line) in self.code_lines.iter().enumerate() {
            if line.to_lowercase().contains(&lower_query) {
                self.search_matches.push(idx + 1);
            }
        }

        if self.search_matches.is_empty() {
            self.status_message = format!("Pattern not found: '{}'", query);
            return;
        }

        if let Some(pos) = self
            .search_matches
            .iter()
            .position(|&l| l >= self.cursor_line)
        {
            self.search_match_index = pos;
        } else {
            self.search_match_index = 0;
        }

        let target_line = self.search_matches[self.search_match_index];
        self.goto_line(target_line);
        self.status_message = format!(
            "Search match {}/{} for '{}' on line {}",
            self.search_match_index + 1,
            self.search_matches.len(),
            query,
            target_line
        );
    }

    pub fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            if let Some(query) = self.last_search_query.clone() {
                self.search_text(&query);
            } else {
                self.status_message = "No active search query".to_string();
            }
            return;
        }

        self.search_match_index = (self.search_match_index + 1) % self.search_matches.len();
        let target_line = self.search_matches[self.search_match_index];
        self.goto_line(target_line);
        if let Some(query) = &self.last_search_query {
            self.status_message = format!(
                "Search match {}/{} for '{}' on line {}",
                self.search_match_index + 1,
                self.search_matches.len(),
                query,
                target_line
            );
        }
    }

    pub fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            if let Some(query) = self.last_search_query.clone() {
                self.search_text(&query);
            } else {
                self.status_message = "No active search query".to_string();
            }
            return;
        }

        if self.search_match_index == 0 {
            self.search_match_index = self.search_matches.len() - 1;
        } else {
            self.search_match_index -= 1;
        }

        let target_line = self.search_matches[self.search_match_index];
        self.goto_line(target_line);
        if let Some(query) = &self.last_search_query {
            self.status_message = format!(
                "Search match {}/{} for '{}' on line {}",
                self.search_match_index + 1,
                self.search_matches.len(),
                query,
                target_line
            );
        }
    }

    pub fn open_symbol_prompt(&mut self) {
        let source_code = self.code_lines.join("\n");
        let file_path = self.current_file_path().unwrap_or_default();
        self.symbol_matches =
            crate::treesitter::extract_symbols(&self.grammar_registry, &file_path, &source_code);
        self.symbol_selected = 0;
        self.input_buffer.clear();
        self.input_prompt = Some(InputPrompt::SearchSymbol);
        self.status_message = format!(
            "Search symbol ({} symbols found):",
            self.symbol_matches.len()
        );
    }

    pub fn filtered_symbols(&self, filter: &str) -> Vec<crate::treesitter::SymbolItem> {
        if filter.is_empty() {
            self.symbol_matches.clone()
        } else {
            let lower = filter.to_lowercase();
            self.symbol_matches
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&lower))
                .cloned()
                .collect()
        }
    }

    pub fn handle_input_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.input_prompt = None;
                self.input_buffer.clear();
                self.status_message = "Cancelled prompt.".to_string();
            }
            KeyCode::Enter => {
                self.submit_input_prompt();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Up => {
                if self.input_prompt == Some(InputPrompt::SearchSymbol) && self.symbol_selected > 0
                {
                    self.symbol_selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.input_prompt == Some(InputPrompt::SearchSymbol) {
                    let filtered = self.filtered_symbols(&self.input_buffer.clone());
                    if !filtered.is_empty() && self.symbol_selected + 1 < filtered.len() {
                        self.symbol_selected += 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    pub fn submit_input_prompt(&mut self) {
        let prompt = match self.input_prompt.take() {
            Some(p) => p,
            None => return,
        };
        let input = self.input_buffer.trim().to_string();
        self.input_buffer.clear();

        match prompt {
            InputPrompt::GotoLine => {
                if let Ok(line_num) = input.parse::<usize>() {
                    self.goto_line(line_num);
                } else {
                    self.status_message = format!("Invalid line number: '{}'", input);
                }
            }
            InputPrompt::SearchText => {
                self.search_text(&input);
            }
            InputPrompt::SearchSymbol => {
                let filtered = self.filtered_symbols(&input);
                if let Some(item) = filtered.get(self.symbol_selected) {
                    let line = item.line_number;
                    self.goto_line(line);
                } else if !filtered.is_empty() {
                    let line = filtered[0].line_number;
                    self.goto_line(line);
                } else {
                    self.status_message = format!("No matching symbol for '{}'", input);
                }
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
        assert!(!app.in_alternate_screen);
    }

    #[test]
    fn test_in_alternate_screen_state_tracking() {
        let mut app = AppState::new(PathBuf::from("/test/repo"));
        assert!(!app.in_alternate_screen);

        app.in_alternate_screen = true;
        assert!(app.in_alternate_screen);

        app.exit_message = Some("Conflict exited".to_string());
        app.in_alternate_screen = false;
        assert!(!app.in_alternate_screen);
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

    #[test]
    fn test_code_viewer_move_selection_updates_viewport_scroll() {
        let mut app = AppState::new(PathBuf::from("."));
        app.active_panel = ActivePanel::CodeViewer;
        app.code_lines = (1..=30).map(|i| format!("line {}", i)).collect();
        app.code_viewport_height = 10;
        app.scrolloff = 2;

        assert_eq!(app.cursor_line, 1);
        assert_eq!(app.code_scroll_offset, 0);

        // Move down past viewport height
        for _ in 0..15 {
            app.move_selection_down();
        }
        assert_eq!(app.cursor_line, 16);
        // Scrolloff = 2, so offset should update automatically on move_selection_down
        assert!(app.code_scroll_offset > 0);

        let scrolled_offset = app.code_scroll_offset;

        // Move back up
        for _ in 0..10 {
            app.move_selection_up();
        }
        assert_eq!(app.cursor_line, 6);
        assert!(app.code_scroll_offset < scrolled_offset);
    }

    #[test]
    fn test_effective_cursor_line_in_diff_view() {
        let mut app = AppState::new(PathBuf::from("."));
        app.code_lines = vec![
            "diff --git a/main.rs b/main.rs".into(),
            "--- a/main.rs".into(),
            "+++ b/main.rs".into(),
            "@@ -10,3 +10,4 @@".into(),
            " fn main() {".into(),       // diff line 5 -> line 10
            "-    old_line();".into(),   // diff line 6 -> old line 11 (deleted)
            "+    new_line_1();".into(), // diff line 7 -> new line 11 (added)
            "+    new_line_2();".into(), // diff line 8 -> new line 12 (added)
            " }".into(),                 // diff line 9 -> line 13
        ];

        app.cursor_line = 5;
        assert_eq!(app.effective_cursor_line(), 10);

        app.cursor_line = 6; // Deleted line
        assert_eq!(app.effective_cursor_line(), 11);

        app.cursor_line = 7; // Added line 1
        assert_eq!(app.effective_cursor_line(), 11);

        app.cursor_line = 8; // Added line 2
        assert_eq!(app.effective_cursor_line(), 12);

        app.cursor_line = 9; // Context line
        assert_eq!(app.effective_cursor_line(), 13);
    }

    #[test]
    fn test_open_file_at_line() {
        let mut app = AppState::new(PathBuf::from("."));
        app.files = vec!["src/main.rs".into(), "Cargo.toml".into()];
        app.code_lines = vec![
            "line 1".into(),
            "line 2".into(),
            "line 3".into(),
            "line 4".into(),
        ];

        app.open_file_at_line("src/main.rs", Some(3));
        assert_eq!(app.active_file, Some("src/main.rs".to_string()));
        assert_eq!(app.active_panel, ActivePanel::CodeViewer);
        assert_eq!(app.cursor_line, 3);
    }

    #[test]
    fn test_set_theme_colors() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.theme_bg, None);
        assert_eq!(app.theme_fg, None);

        app.set_theme_colors(
            Some(ratatui::style::Color::Rgb(30, 30, 46)),
            Some(ratatui::style::Color::Rgb(205, 214, 244)),
        );
        assert_eq!(app.theme_bg, Some(ratatui::style::Color::Rgb(30, 30, 46)));
        assert_eq!(
            app.theme_fg,
            Some(ratatui::style::Color::Rgb(205, 214, 244))
        );
    }
}
