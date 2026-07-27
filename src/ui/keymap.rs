use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::{KeyBindingConfig, KeymapConfig, ScopeKeymapConfig};

/// Representation of a single keystroke
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyStroke {
    Char(char),
    Esc,
    Tab,
    Enter,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Ctrl(char),
}

impl KeyStroke {
    /// Parse key string like "j", "]", "<Esc>", "<Tab>", "<CR>", "<C-d>"
    pub fn parse(s: &str) -> Vec<KeyStroke> {
        let mut strokes = Vec::new();
        let mut chars = s.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c == '<' {
                let rest: String = chars.clone().collect();
                if let Some(end_idx) = rest.find('>') {
                    let tag = &rest[..=end_idx];
                    let parsed = if tag.eq_ignore_ascii_case("<esc>") {
                        Some(KeyStroke::Esc)
                    } else if tag.eq_ignore_ascii_case("<tab>") {
                        Some(KeyStroke::Tab)
                    } else if tag.eq_ignore_ascii_case("<cr>")
                        || tag.eq_ignore_ascii_case("<enter>")
                    {
                        Some(KeyStroke::Enter)
                    } else if tag.eq_ignore_ascii_case("<up>") {
                        Some(KeyStroke::Up)
                    } else if tag.eq_ignore_ascii_case("<down>") {
                        Some(KeyStroke::Down)
                    } else if tag.eq_ignore_ascii_case("<left>") {
                        Some(KeyStroke::Left)
                    } else if tag.eq_ignore_ascii_case("<right>") {
                        Some(KeyStroke::Right)
                    } else if tag.eq_ignore_ascii_case("<pageup>") {
                        Some(KeyStroke::PageUp)
                    } else if tag.eq_ignore_ascii_case("<pagedown>") {
                        Some(KeyStroke::PageDown)
                    } else if tag.to_ascii_lowercase().starts_with("<c-") && tag.len() == 5 {
                        tag.chars()
                            .nth(3)
                            .map(|ch| KeyStroke::Ctrl(ch.to_ascii_lowercase()))
                    } else {
                        None
                    };

                    if let Some(stroke) = parsed {
                        strokes.push(stroke);
                        for _ in 0..tag.chars().count() {
                            chars.next();
                        }
                        continue;
                    }
                }
            }

            strokes.push(KeyStroke::Char(c));
            chars.next();
        }

        strokes
    }
}

impl TryFrom<crossterm::event::KeyEvent> for KeyStroke {
    type Error = ();

    fn try_from(event: crossterm::event::KeyEvent) -> Result<Self, Self::Error> {
        use crossterm::event::{KeyCode, KeyModifiers};

        if event.modifiers.contains(KeyModifiers::CONTROL) {
            if let KeyCode::Char(c) = event.code {
                return Ok(KeyStroke::Ctrl(c.to_ascii_lowercase()));
            }
        }

        match event.code {
            KeyCode::Char(c) => Ok(KeyStroke::Char(c)),
            KeyCode::Esc => Ok(KeyStroke::Esc),
            KeyCode::Tab => Ok(KeyStroke::Tab),
            KeyCode::Enter => Ok(KeyStroke::Enter),
            KeyCode::Up => Ok(KeyStroke::Up),
            KeyCode::Down => Ok(KeyStroke::Down),
            KeyCode::Left => Ok(KeyStroke::Left),
            KeyCode::Right => Ok(KeyStroke::Right),
            KeyCode::PageUp => Ok(KeyStroke::PageUp),
            KeyCode::PageDown => Ok(KeyStroke::PageDown),
            _ => Err(()),
        }
    }
}

/// Sequence of keystrokes representing a keybinding (e.g. "]" or "]f")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence(pub Vec<KeyStroke>);

/// All possible actions in Git-tardis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    ToggleHelp,
    ToggleSplashscreen,
    ToggleFocus,
    SetSidebarView(usize), // 1, 2, 3
    CycleNavMode,

    MoveUp,
    MoveDown,
    Select,

    // Scrolling and page navigation actions
    HalfPageDown,
    HalfPageUp,
    PageDown,
    PageUp,
    ScrollLineDown,
    ScrollLineUp,
    CenterCursor,
    CursorTop,
    CursorBottom,

    // Timeline navigation actions
    JumpNextAuto,     // Default active mode jump
    JumpPrevAuto,     // Default active mode jump
    JumpNextCommit,   // Force commit mode jump
    JumpPrevCommit,   // Force commit mode jump
    JumpNextFile,     // Force file mode jump
    JumpPrevFile,     // Force file mode jump
    JumpNextFunction, // Force function mode jump
    JumpPrevFunction, // Force function mode jump
    JumpNextLine,     // Force line mode jump
    JumpPrevLine,     // Force line mode jump

    InlineRewrite,
    EditHere,

    // Markdown rendering action
    ToggleMarkdownFormat,

    // Tree-based File Explorer actions
    ExpandFolder,
    CollapseFolder,
    ToggleFolder,
}

/// Active panel focus scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Sidebar,
    CodeViewer,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MatchResult {
    FullMatch(Action),
    AmbiguousMatch(Action), // Matches an action, but is also a prefix for longer sequences
    PrefixMatch,
    NoMatch,
}

/// Registry mapping scope -> key sequence -> action
#[derive(Default, Debug, Clone)]
pub struct KeymapRegistry {
    maps: HashMap<Scope, HashMap<KeySequence, Action>>,
}

impl KeymapRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.load_defaults();
        registry
    }

    /// Load default Git-tardis keybindings
    pub fn load_defaults(&mut self) {
        // Global scope
        self.bind(Scope::Global, "q", Action::Quit);
        self.bind(Scope::Global, "<Esc>", Action::Quit);
        self.bind(Scope::Global, "?", Action::ToggleHelp);
        self.bind(Scope::Global, "S", Action::ToggleSplashscreen);
        self.bind(Scope::Global, "M", Action::ToggleMarkdownFormat);
        self.bind(Scope::Global, "<Tab>", Action::ToggleFocus);
        self.bind(Scope::Global, "h", Action::ToggleFocus);
        self.bind(Scope::Global, "l", Action::ToggleFocus);
        self.bind(Scope::Global, "<Left>", Action::ToggleFocus);
        self.bind(Scope::Global, "<Right>", Action::ToggleFocus);
        self.bind(Scope::Global, "1", Action::SetSidebarView(1));
        self.bind(Scope::Global, "2", Action::SetSidebarView(2));
        self.bind(Scope::Global, "3", Action::SetSidebarView(3));
        self.bind(Scope::Global, "4", Action::SetSidebarView(4));
        self.bind(Scope::Global, "m", Action::CycleNavMode);

        // Page navigation shortcuts
        self.bind(Scope::Global, "<C-d>", Action::HalfPageDown);
        self.bind(Scope::Global, "<C-u>", Action::HalfPageUp);
        self.bind(Scope::Global, "<C-f>", Action::PageDown);
        self.bind(Scope::Global, "<C-b>", Action::PageUp);
        self.bind(Scope::Global, "<PageDown>", Action::PageDown);
        self.bind(Scope::Global, "<PageUp>", Action::PageUp);

        // Sidebar scope
        self.bind(Scope::Sidebar, "j", Action::MoveDown);
        self.bind(Scope::Sidebar, "k", Action::MoveUp);
        self.bind(Scope::Sidebar, "<Down>", Action::MoveDown);
        self.bind(Scope::Sidebar, "<Up>", Action::MoveUp);
        self.bind(Scope::Sidebar, "<CR>", Action::Select);
        self.bind(Scope::Sidebar, "l", Action::Select);
        self.bind(Scope::Sidebar, "]", Action::JumpNextAuto);
        self.bind(Scope::Sidebar, "[", Action::JumpPrevAuto);
        self.bind(Scope::Sidebar, "<Right>", Action::ExpandFolder);
        self.bind(Scope::Sidebar, "<Left>", Action::CollapseFolder);
        self.bind(Scope::Sidebar, "h", Action::CollapseFolder);
        self.bind(Scope::Sidebar, " ", Action::ToggleFolder);

        // Code Viewer scope
        self.bind(Scope::CodeViewer, "j", Action::MoveDown);
        self.bind(Scope::CodeViewer, "k", Action::MoveUp);
        self.bind(Scope::CodeViewer, "<Down>", Action::MoveDown);
        self.bind(Scope::CodeViewer, "<Up>", Action::MoveUp);

        // Timeline navigation shortcuts
        self.bind(Scope::CodeViewer, "]", Action::JumpNextAuto);
        self.bind(Scope::CodeViewer, "[", Action::JumpPrevAuto);

        // Line and viewport repositioning shortcuts
        self.bind(Scope::CodeViewer, "<C-e>", Action::ScrollLineDown);
        self.bind(Scope::CodeViewer, "<C-y>", Action::ScrollLineUp);
        self.bind(Scope::CodeViewer, "zz", Action::CenterCursor);
        self.bind(Scope::CodeViewer, "zt", Action::CursorTop);
        self.bind(Scope::CodeViewer, "zb", Action::CursorBottom);

        self.bind(Scope::CodeViewer, "e", Action::InlineRewrite);
        self.bind(Scope::CodeViewer, "E", Action::EditHere);
    }

    pub fn bind(&mut self, scope: Scope, key_str: &str, action: Action) {
        let strokes = KeyStroke::parse(key_str);
        self.maps
            .entry(scope)
            .or_default()
            .insert(KeySequence(strokes), action);
    }

    /// Resolve key sequence against context scope hierarchy (ActiveScope -> GlobalScope)
    pub fn resolve(&self, active_scope: Scope, pending: &[KeyStroke]) -> MatchResult {
        // 1. Check active scope
        let res = self.check_scope(active_scope, pending);
        if res != MatchResult::NoMatch {
            return res;
        }

        // 2. Check global scope if active scope didn't match
        if active_scope != Scope::Global {
            return self.check_scope(Scope::Global, pending);
        }

        MatchResult::NoMatch
    }

    fn check_scope(&self, scope: Scope, pending: &[KeyStroke]) -> MatchResult {
        let empty_map = HashMap::new();
        let map = self.maps.get(&scope).unwrap_or(&empty_map);

        let target_seq = KeySequence(pending.to_vec());
        let full_match = map.get(&target_seq);

        let is_prefix = map
            .keys()
            .any(|seq| seq.0.starts_with(pending) && seq.0.len() > pending.len());

        if let Some(action) = full_match {
            if is_prefix {
                MatchResult::AmbiguousMatch(action.clone())
            } else {
                MatchResult::FullMatch(action.clone())
            }
        } else if is_prefix {
            MatchResult::PrefixMatch
        } else {
            MatchResult::NoMatch
        }
    }

    /// Apply configuration struct to registry
    pub fn apply_config(&mut self, config: &KeymapConfig) {
        if let Some(global) = &config.global {
            self.apply_scope_config(Scope::Global, global);
        }
        if let Some(sidebar) = &config.sidebar {
            self.apply_scope_config(Scope::Sidebar, sidebar);
        }
        if let Some(code_viewer) = &config.code_viewer {
            self.apply_scope_config(Scope::CodeViewer, code_viewer);
        }
    }

    /// Load configuration overrides from TOML string
    pub fn apply_toml_config(&mut self, toml_str: &str) -> Result<(), String> {
        let cfg: KeymapConfig =
            toml::from_str(toml_str).map_err(|e| format!("Failed to parse keymap TOML: {}", e))?;
        self.apply_config(&cfg);
        Ok(())
    }

    fn apply_scope_config(&mut self, scope: Scope, mappings: &ScopeKeymapConfig) {
        let bind_list = |registry: &mut KeymapRegistry,
                         scope: Scope,
                         keys: &Option<KeyBindingConfig>,
                         action: Action| {
            if let Some(cfg) = keys {
                for key_str in cfg.clone().into_vec() {
                    registry.bind(scope, &key_str, action.clone());
                }
            }
        };

        bind_list(self, scope, &mappings.quit, Action::Quit);
        bind_list(self, scope, &mappings.toggle_help, Action::ToggleHelp);
        bind_list(
            self,
            scope,
            &mappings.toggle_splashscreen,
            Action::ToggleSplashscreen,
        );
        bind_list(self, scope, &mappings.toggle_focus, Action::ToggleFocus);
        bind_list(self, scope, &mappings.cycle_nav_mode, Action::CycleNavMode);
        bind_list(self, scope, &mappings.move_up, Action::MoveUp);
        bind_list(self, scope, &mappings.move_down, Action::MoveDown);
        bind_list(self, scope, &mappings.select, Action::Select);
        bind_list(self, scope, &mappings.half_page_down, Action::HalfPageDown);
        bind_list(self, scope, &mappings.half_page_up, Action::HalfPageUp);
        bind_list(self, scope, &mappings.page_down, Action::PageDown);
        bind_list(self, scope, &mappings.page_up, Action::PageUp);
        bind_list(
            self,
            scope,
            &mappings.scroll_line_down,
            Action::ScrollLineDown,
        );
        bind_list(self, scope, &mappings.scroll_line_up, Action::ScrollLineUp);
        bind_list(self, scope, &mappings.center_cursor, Action::CenterCursor);
        bind_list(self, scope, &mappings.cursor_top, Action::CursorTop);
        bind_list(self, scope, &mappings.cursor_bottom, Action::CursorBottom);
        bind_list(self, scope, &mappings.jump_next, Action::JumpNextAuto);
        bind_list(self, scope, &mappings.jump_prev, Action::JumpPrevAuto);
        bind_list(
            self,
            scope,
            &mappings.jump_next_commit,
            Action::JumpNextCommit,
        );
        bind_list(
            self,
            scope,
            &mappings.jump_prev_commit,
            Action::JumpPrevCommit,
        );
        bind_list(self, scope, &mappings.jump_next_file, Action::JumpNextFile);
        bind_list(self, scope, &mappings.jump_prev_file, Action::JumpPrevFile);
        bind_list(
            self,
            scope,
            &mappings.jump_next_function,
            Action::JumpNextFunction,
        );
        bind_list(
            self,
            scope,
            &mappings.jump_prev_function,
            Action::JumpPrevFunction,
        );
        bind_list(self, scope, &mappings.jump_next_line, Action::JumpNextLine);
        bind_list(self, scope, &mappings.jump_prev_line, Action::JumpPrevLine);
        bind_list(self, scope, &mappings.inline_rewrite, Action::InlineRewrite);
        bind_list(self, scope, &mappings.edit_here, Action::EditHere);
        bind_list(
            self,
            scope,
            &mappings.toggle_markdown_format,
            Action::ToggleMarkdownFormat,
        );
    }
}

/// State machine for processing keystrokes with sequence timeouts
pub struct KeyDispatcher {
    registry: KeymapRegistry,
    pending: Vec<KeyStroke>,
    last_key_time: Option<Instant>,
    timeout: Duration,
}

impl KeyDispatcher {
    pub fn new(registry: KeymapRegistry) -> Self {
        Self {
            registry,
            pending: Vec::new(),
            last_key_time: None,
            timeout: Duration::from_millis(500),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn registry(&self) -> &KeymapRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut KeymapRegistry {
        &mut self.registry
    }

    pub fn pending(&self) -> &[KeyStroke] {
        &self.pending
    }

    pub fn handle_key(&mut self, stroke: KeyStroke, active_scope: Scope) -> Option<Action> {
        self.pending.push(stroke);
        self.last_key_time = Some(Instant::now());

        match self.registry.resolve(active_scope, &self.pending) {
            MatchResult::FullMatch(action) => {
                self.pending.clear();
                self.last_key_time = None;
                Some(action)
            }
            MatchResult::AmbiguousMatch(_action) => {
                // Buffer keystroke, wait for potential sequence completion or timeout
                None
            }
            MatchResult::PrefixMatch => {
                // Buffer keystroke, wait for continuation
                None
            }
            MatchResult::NoMatch => {
                if self.pending.len() > 1 {
                    let prefix = self.pending[..self.pending.len() - 1].to_vec();
                    if let MatchResult::AmbiguousMatch(fallback_action) =
                        self.registry.resolve(active_scope, &prefix)
                    {
                        // Pop non-matching stroke and re-evaluate
                        let last_stroke = self.pending.pop().unwrap();
                        self.pending.clear();
                        self.pending.push(last_stroke);
                        self.last_key_time = Some(Instant::now());

                        let next_match = self.registry.resolve(active_scope, &self.pending);
                        match next_match {
                            MatchResult::FullMatch(action) => {
                                self.pending.clear();
                                self.last_key_time = None;
                                Some(action)
                            }
                            MatchResult::AmbiguousMatch(_) | MatchResult::PrefixMatch => {
                                Some(fallback_action)
                            }
                            MatchResult::NoMatch => {
                                self.pending.clear();
                                self.last_key_time = None;
                                Some(fallback_action)
                            }
                        }
                    } else {
                        self.pending.clear();
                        self.last_key_time = None;
                        None
                    }
                } else {
                    self.pending.clear();
                    self.last_key_time = None;
                    None
                }
            }
        }
    }

    pub fn handle_event(
        &mut self,
        event: crossterm::event::KeyEvent,
        active_scope: Scope,
    ) -> Option<Action> {
        if let Ok(stroke) = KeyStroke::try_from(event) {
            self.handle_key(stroke, active_scope)
        } else {
            None
        }
    }

    pub fn check_timeout(&mut self, active_scope: Scope) -> Option<Action> {
        if self.pending.is_empty() {
            return None;
        }

        if let Some(last_time) = self.last_key_time {
            if last_time.elapsed() >= self.timeout {
                let match_res = self.registry.resolve(active_scope, &self.pending);
                self.pending.clear();
                self.last_key_time = None;
                if let MatchResult::AmbiguousMatch(action) = match_res {
                    return Some(action);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_stroke_parsing() {
        assert_eq!(KeyStroke::parse("j"), vec![KeyStroke::Char('j')]);
        assert_eq!(
            KeyStroke::parse("]f"),
            vec![KeyStroke::Char(']'), KeyStroke::Char('f')]
        );
        assert_eq!(KeyStroke::parse("<Esc>"), vec![KeyStroke::Esc]);
        assert_eq!(KeyStroke::parse("<Tab>"), vec![KeyStroke::Tab]);
        assert_eq!(KeyStroke::parse("<CR>"), vec![KeyStroke::Enter]);
        assert_eq!(KeyStroke::parse("<C-d>"), vec![KeyStroke::Ctrl('d')]);
    }

    #[test]
    fn test_sequence_resolution_and_scope_override() {
        let registry = KeymapRegistry::new();

        // Single stroke in sidebar
        let res = registry.resolve(Scope::Sidebar, &[KeyStroke::Char('j')]);
        assert_eq!(res, MatchResult::FullMatch(Action::MoveDown));

        // Fallback to global
        let res_global = registry.resolve(Scope::Sidebar, &[KeyStroke::Tab]);
        assert_eq!(res_global, MatchResult::FullMatch(Action::ToggleFocus));

        // Single stroke jump next in code viewer matches immediately
        let res_jump = registry.resolve(Scope::CodeViewer, &[KeyStroke::Char(']')]);
        assert_eq!(res_jump, MatchResult::FullMatch(Action::JumpNextAuto));
    }

    #[test]
    fn test_dispatcher_sequence_matching() {
        let mut registry = KeymapRegistry::new();
        // Bind multi-stroke sequence ]f for testing custom configuration
        registry.bind(Scope::CodeViewer, "]f", Action::JumpNextFunction);
        let mut dispatcher = KeyDispatcher::new(registry);

        // Step 1: Send ']' -> Ambiguous because ]f exists
        let act1 = dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
        assert_eq!(act1, None);

        // Step 2: Send 'f' -> completes ']f'
        let act2 = dispatcher.handle_key(KeyStroke::Char('f'), Scope::CodeViewer);
        assert_eq!(act2, Some(Action::JumpNextFunction));
        assert!(dispatcher.pending().is_empty());
    }

    #[test]
    fn test_dispatcher_ambiguous_timeout() {
        let mut registry = KeymapRegistry::new();
        // Bind multi-stroke sequence ]f for testing ambiguous timeout
        registry.bind(Scope::CodeViewer, "]f", Action::JumpNextFunction);
        let mut dispatcher = KeyDispatcher::new(registry).with_timeout(Duration::from_millis(10));

        // Send ']'
        let act1 = dispatcher.handle_key(KeyStroke::Char(']'), Scope::CodeViewer);
        assert_eq!(act1, None);

        std::thread::sleep(Duration::from_millis(15));

        let act2 = dispatcher.check_timeout(Scope::CodeViewer);
        assert_eq!(act2, Some(Action::JumpNextAuto));
        assert!(dispatcher.pending().is_empty());
    }

    #[test]
    fn test_toml_keymap_override() {
        let mut registry = KeymapRegistry::new();
        let toml_str = r#"
        [global]
        quit = "<C-q>"
        
        [code_viewer]
        jump_next_function = "]F"
        "#;

        registry.apply_toml_config(toml_str).unwrap();

        let res_quit = registry.resolve(Scope::Global, &[KeyStroke::Ctrl('q')]);
        assert_eq!(res_quit, MatchResult::FullMatch(Action::Quit));

        let res_func = registry.resolve(
            Scope::CodeViewer,
            &[KeyStroke::Char(']'), KeyStroke::Char('F')],
        );
        assert_eq!(res_func, MatchResult::FullMatch(Action::JumpNextFunction));
    }
}
