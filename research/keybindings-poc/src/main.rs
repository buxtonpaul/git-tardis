use serde::Deserialize;
use std::collections::HashMap;

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
    Ctrl(char),
}

impl KeyStroke {
    /// Parse key string like "j", "]", "<Esc>", "<Tab>", "<CR>", "<C-d>"
    pub fn parse(s: &str) -> Vec<KeyStroke> {
        let mut strokes = Vec::new();
        let mut chars = s.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c == '<' {
                let s_rest: String = chars.by_ref().collect();
                if s_rest.eq_ignore_ascii_case("<esc>") {
                    strokes.push(KeyStroke::Esc);
                } else if s_rest.eq_ignore_ascii_case("<tab>") {
                    strokes.push(KeyStroke::Tab);
                } else if s_rest.eq_ignore_ascii_case("<cr>") || s_rest.eq_ignore_ascii_case("<enter>") {
                    strokes.push(KeyStroke::Enter);
                } else if s_rest.eq_ignore_ascii_case("<up>") {
                    strokes.push(KeyStroke::Up);
                } else if s_rest.eq_ignore_ascii_case("<down>") {
                    strokes.push(KeyStroke::Down);
                } else if s_rest.eq_ignore_ascii_case("<left>") {
                    strokes.push(KeyStroke::Left);
                } else if s_rest.eq_ignore_ascii_case("<right>") {
                    strokes.push(KeyStroke::Right);
                } else if s_rest.starts_with("<C-") && s_rest.ends_with('>') && s_rest.len() == 5 {
                    if let Some(ctrl_char) = s_rest.chars().nth(3) {
                        strokes.push(KeyStroke::Ctrl(ctrl_char));
                    }
                } else {
                    for ch in s_rest.chars() {
                        strokes.push(KeyStroke::Char(ch));
                    }
                }
            } else {
                strokes.push(KeyStroke::Char(c));
                chars.next();
            }
        }
        strokes
    }
}

/// Sequence of keystrokes representing a keybinding (e.g. "]" or "]f")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySequence(pub Vec<KeyStroke>);

/// All possible actions in Git-tardis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    ToggleFocus,
    SetSidebarView(usize), // 1, 2, 3
    CycleNavMode,
    
    MoveUp,
    MoveDown,
    Select,

    // Timeline navigation actions
    JumpNextAuto,       // Default active mode jump
    JumpPrevAuto,       // Default active mode jump
    JumpNextFile,       // Force file mode jump
    JumpPrevFile,       // Force file mode jump
    JumpNextFunction,   // Force function mode jump
    JumpPrevFunction,   // Force function mode jump
    JumpNextLine,       // Force line mode jump
    JumpPrevLine,       // Force line mode jump

    InlineRewrite,
    EditHere,
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
#[derive(Default)]
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
        self.bind(Scope::Global, "<Tab>", Action::ToggleFocus);
        self.bind(Scope::Global, "1", Action::SetSidebarView(1));
        self.bind(Scope::Global, "2", Action::SetSidebarView(2));
        self.bind(Scope::Global, "3", Action::SetSidebarView(3));
        self.bind(Scope::Global, "m", Action::CycleNavMode);

        // Sidebar scope
        self.bind(Scope::Sidebar, "j", Action::MoveDown);
        self.bind(Scope::Sidebar, "k", Action::MoveUp);
        self.bind(Scope::Sidebar, "<Down>", Action::MoveDown);
        self.bind(Scope::Sidebar, "<Up>", Action::MoveUp);
        self.bind(Scope::Sidebar, "<CR>", Action::Select);

        // Code Viewer scope
        self.bind(Scope::CodeViewer, "j", Action::MoveDown);
        self.bind(Scope::CodeViewer, "k", Action::MoveUp);
        self.bind(Scope::CodeViewer, "<Down>", Action::MoveDown);
        self.bind(Scope::CodeViewer, "<Up>", Action::MoveUp);
        
        // Mode-specific timeline navigation shortcuts
        self.bind(Scope::CodeViewer, "]", Action::JumpNextAuto);
        self.bind(Scope::CodeViewer, "[", Action::JumpPrevAuto);
        
        self.bind(Scope::CodeViewer, "]m", Action::JumpNextFile);
        self.bind(Scope::CodeViewer, "[m", Action::JumpPrevFile);

        self.bind(Scope::CodeViewer, "]f", Action::JumpNextFunction);
        self.bind(Scope::CodeViewer, "[f", Action::JumpPrevFunction);

        self.bind(Scope::CodeViewer, "]l", Action::JumpNextLine);
        self.bind(Scope::CodeViewer, "[l", Action::JumpPrevLine);

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

        let is_prefix = map.keys().any(|seq| {
            seq.0.starts_with(pending) && seq.0.len() > pending.len()
        });

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

    /// Load configuration overrides from TOML string
    pub fn apply_toml_config(&mut self, toml_str: &str) -> Result<(), String> {
        let cfg: KeymapConfig = toml::from_str(toml_str)
            .map_err(|e| format!("Failed to parse keymap TOML: {}", e))?;

        if let Some(global) = cfg.global {
            self.apply_scope_config(Scope::Global, global);
        }
        if let Some(sidebar) = cfg.sidebar {
            self.apply_scope_config(Scope::Sidebar, sidebar);
        }
        if let Some(code_viewer) = cfg.code_viewer {
            self.apply_scope_config(Scope::CodeViewer, code_viewer);
        }

        Ok(())
    }

    fn apply_scope_config(&mut self, scope: Scope, mappings: ScopeKeymapConfig) {
        let bind_list = |registry: &mut KeymapRegistry, scope: Scope, keys: Option<KeyBindingConfig>, action: Action| {
            if let Some(cfg) = keys {
                for key_str in cfg.into_vec() {
                    registry.bind(scope, &key_str, action.clone());
                }
            }
        };

        bind_list(self, scope, mappings.quit, Action::Quit);
        bind_list(self, scope, mappings.toggle_focus, Action::ToggleFocus);
        bind_list(self, scope, mappings.cycle_nav_mode, Action::CycleNavMode);
        bind_list(self, scope, mappings.move_up, Action::MoveUp);
        bind_list(self, scope, mappings.move_down, Action::MoveDown);
        bind_list(self, scope, mappings.select, Action::Select);
        bind_list(self, scope, mappings.jump_next, Action::JumpNextAuto);
        bind_list(self, scope, mappings.jump_prev, Action::JumpPrevAuto);
        bind_list(self, scope, mappings.jump_next_file, Action::JumpNextFile);
        bind_list(self, scope, mappings.jump_prev_file, Action::JumpPrevFile);
        bind_list(self, scope, mappings.jump_next_function, Action::JumpNextFunction);
        bind_list(self, scope, mappings.jump_prev_function, Action::JumpPrevFunction);
        bind_list(self, scope, mappings.jump_next_line, Action::JumpNextLine);
        bind_list(self, scope, mappings.jump_prev_line, Action::JumpPrevLine);
        bind_list(self, scope, mappings.inline_rewrite, Action::InlineRewrite);
        bind_list(self, scope, mappings.edit_here, Action::EditHere);
    }
}

/// TOML Configuration Serialization
#[derive(Debug, Deserialize, Default)]
pub struct KeymapConfig {
    pub global: Option<ScopeKeymapConfig>,
    pub sidebar: Option<ScopeKeymapConfig>,
    pub code_viewer: Option<ScopeKeymapConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ScopeKeymapConfig {
    pub quit: Option<KeyBindingConfig>,
    pub toggle_focus: Option<KeyBindingConfig>,
    pub cycle_nav_mode: Option<KeyBindingConfig>,
    pub move_up: Option<KeyBindingConfig>,
    pub move_down: Option<KeyBindingConfig>,
    pub select: Option<KeyBindingConfig>,
    pub jump_next: Option<KeyBindingConfig>,
    pub jump_prev: Option<KeyBindingConfig>,
    pub jump_next_file: Option<KeyBindingConfig>,
    pub jump_prev_file: Option<KeyBindingConfig>,
    pub jump_next_function: Option<KeyBindingConfig>,
    pub jump_prev_function: Option<KeyBindingConfig>,
    pub jump_next_line: Option<KeyBindingConfig>,
    pub jump_prev_line: Option<KeyBindingConfig>,
    pub inline_rewrite: Option<KeyBindingConfig>,
    pub edit_here: Option<KeyBindingConfig>,
}

/// Allows either a single string ("q") or array of strings (["q", "<Esc>"])
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum KeyBindingConfig {
    Single(String),
    Multiple(Vec<String>),
}

impl KeyBindingConfig {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            KeyBindingConfig::Single(s) => vec![s],
            KeyBindingConfig::Multiple(v) => v,
        }
    }
}

fn main() {
    println!("=== Configurable Keybindings & Mode Shortcuts PoC ===");

    let mut registry = KeymapRegistry::new();

    // 1. Test Single vs Multi-Key Sequence Resolution in CodeViewer Scope
    println!("\n--- 1. Multi-Key Sequence Resolution ---");
    let seq_single_jump = vec![KeyStroke::Char(']')];
    let res1 = registry.resolve(Scope::CodeViewer, &seq_single_jump);
    println!("Key sequence ']' in CodeViewer -> MatchResult: {:?}", res1);
    assert_eq!(res1, MatchResult::AmbiguousMatch(Action::JumpNextAuto)); // Because ']' is prefix to ']f', ']l', ']m'!

    let seq_func_jump = vec![KeyStroke::Char(']'), KeyStroke::Char('f')];
    let res2 = registry.resolve(Scope::CodeViewer, &seq_func_jump);
    println!("Key sequence ']f' in CodeViewer -> MatchResult: {:?}", res2);
    assert_eq!(res2, MatchResult::FullMatch(Action::JumpNextFunction));

    let seq_line_jump = vec![KeyStroke::Char(']'), KeyStroke::Char('l')];
    let res3 = registry.resolve(Scope::CodeViewer, &seq_line_jump);
    println!("Key sequence ']l' in CodeViewer -> MatchResult: {:?}", res3);
    assert_eq!(res3, MatchResult::FullMatch(Action::JumpNextLine));

    // 2. Test Scope Hierarchy (Sidebar vs CodeViewer vs Global)
    println!("\n--- 2. Scope Hierarchy Resolution ---");
    let enter_key = vec![KeyStroke::Enter];
    
    // In Sidebar scope, Enter -> Action::Select
    let res_sb = registry.resolve(Scope::Sidebar, &enter_key);
    println!("'<CR>' in Sidebar -> {:?}", res_sb);
    assert_eq!(res_sb, MatchResult::FullMatch(Action::Select));

    // In Global scope, Tab -> Action::ToggleFocus
    let tab_key = vec![KeyStroke::Tab];
    let res_global = registry.resolve(Scope::Sidebar, &tab_key); // Should resolve via global fallback!
    println!("'<Tab>' while focused in Sidebar -> {:?}", res_global);
    assert_eq!(res_global, MatchResult::FullMatch(Action::ToggleFocus));

    // 3. Test TOML Custom Overrides
    println!("\n--- 3. Custom TOML Keymaps Override ---");
    let custom_toml = r#"
    [code_viewer]
    jump_next_function = ["]f", "<C-f>"]
    jump_next_line = "]l"
    
    [global]
    quit = ["q", "<Esc>", "<C-c>"]
    "#;

    registry.apply_toml_config(custom_toml).unwrap();

    let ctrl_f = vec![KeyStroke::Ctrl('f')];
    let res_ctrl_f = registry.resolve(Scope::CodeViewer, &ctrl_f);
    println!("Custom binding '<C-f>' in CodeViewer -> {:?}", res_ctrl_f);
    assert_eq!(res_ctrl_f, MatchResult::FullMatch(Action::JumpNextFunction));

    println!("\n=== PoC Executed Successfully ===");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_stroke_parsing() {
        assert_eq!(KeyStroke::parse("j"), vec![KeyStroke::Char('j')]);
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

        // Multi-stroke jump function in code viewer
        let res = registry.resolve(Scope::CodeViewer, &[KeyStroke::Char(']'), KeyStroke::Char('f')]);
        assert_eq!(res, MatchResult::FullMatch(Action::JumpNextFunction));

        // Multi-stroke jump line in code viewer
        let res = registry.resolve(Scope::CodeViewer, &[KeyStroke::Char(']'), KeyStroke::Char('l')]);
        assert_eq!(res, MatchResult::FullMatch(Action::JumpNextLine));

        // Multi-stroke jump file in code viewer
        let res = registry.resolve(Scope::CodeViewer, &[KeyStroke::Char(']'), KeyStroke::Char('m')]);
        assert_eq!(res, MatchResult::FullMatch(Action::JumpNextFile));
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

        let res_func = registry.resolve(Scope::CodeViewer, &[KeyStroke::Char(']'), KeyStroke::Char('F')]);
        assert_eq!(res_func, MatchResult::FullMatch(Action::JumpNextFunction));
    }
}
