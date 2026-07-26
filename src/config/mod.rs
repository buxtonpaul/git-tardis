use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub languages: Option<Vec<LanguageConfig>>,
    pub keymaps: Option<KeymapConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct LanguageConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub library_path: Option<PathBuf>,
    pub symbol_name: Option<String>,
    pub node_kinds: Option<Vec<String>>,
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct KeymapConfig {
    pub global: Option<ScopeKeymapConfig>,
    pub sidebar: Option<ScopeKeymapConfig>,
    pub code_viewer: Option<ScopeKeymapConfig>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct ScopeKeymapConfig {
    pub quit: Option<KeyBindingConfig>,
    pub toggle_focus: Option<KeyBindingConfig>,
    pub cycle_nav_mode: Option<KeyBindingConfig>,
    pub move_up: Option<KeyBindingConfig>,
    pub move_down: Option<KeyBindingConfig>,
    pub select: Option<KeyBindingConfig>,
    pub jump_next: Option<KeyBindingConfig>,
    pub jump_prev: Option<KeyBindingConfig>,
    pub jump_next_commit: Option<KeyBindingConfig>,
    pub jump_prev_commit: Option<KeyBindingConfig>,
    pub jump_next_file: Option<KeyBindingConfig>,
    pub jump_prev_file: Option<KeyBindingConfig>,
    pub jump_next_function: Option<KeyBindingConfig>,
    pub jump_prev_function: Option<KeyBindingConfig>,
    pub jump_next_line: Option<KeyBindingConfig>,
    pub jump_prev_line: Option<KeyBindingConfig>,
    pub inline_rewrite: Option<KeyBindingConfig>,
    pub edit_here: Option<KeyBindingConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

impl Config {
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        toml::from_str(toml_str).map_err(|e| format!("Failed to parse TOML configuration: {}", e))
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("Configuration file does not exist: {:?}", path));
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;
        Self::from_toml(&content)
    }

    pub fn default_config_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("git-tardis")
                .join("config.toml"),
        )
    }

    pub fn load_or_default(custom_path: Option<&Path>) -> Self {
        if let Some(path) = custom_path {
            if let Ok(cfg) = Self::load_from_file(path) {
                return cfg;
            }
        } else if let Some(default_path) = Self::default_config_path() {
            if default_path.exists() {
                if let Ok(cfg) = Self::load_from_file(&default_path) {
                    return cfg;
                }
            }
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config_toml() {
        let toml_str = r#"
        [[languages]]
        name = "rust"
        extensions = ["rs", "rs.in"]
        node_kinds = ["function_item", "closure_expression"]

        [keymaps.global]
        quit = ["q", "<Esc>"]
        
        [keymaps.code_viewer]
        jump_next_function = "]f"
        "#;

        let cfg = Config::from_toml(toml_str).unwrap();
        assert!(cfg.languages.is_some());
        let langs = cfg.languages.unwrap();
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0].name, "rust");

        let keymaps = cfg.keymaps.unwrap();
        let global = keymaps.global.unwrap();
        assert_eq!(
            global.quit,
            Some(KeyBindingConfig::Multiple(vec!["q".into(), "<Esc>".into()]))
        );

        let code_viewer = keymaps.code_viewer.unwrap();
        assert_eq!(
            code_viewer.jump_next_function,
            Some(KeyBindingConfig::Single("]f".into()))
        );
    }

    #[test]
    fn test_keybinding_config_conversion() {
        let single = KeyBindingConfig::Single("j".into());
        assert_eq!(single.into_vec(), vec!["j"]);

        let multiple = KeyBindingConfig::Multiple(vec!["j".into(), "<Down>".into()]);
        assert_eq!(multiple.into_vec(), vec!["j", "<Down>"]);
    }
}
