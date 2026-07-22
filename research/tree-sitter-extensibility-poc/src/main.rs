use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tree_sitter::{Language, Node, Parser, Query};

/// Configuration structure for language definitions (TOML / Lua)
#[derive(Debug, Deserialize, Clone)]
pub struct LanguageConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub library_path: Option<PathBuf>,
    pub symbol_name: Option<String>,
    pub node_kinds: Option<Vec<String>>,
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegistryConfig {
    pub languages: Vec<LanguageConfig>,
}

/// Represents a loaded grammar in memory
pub struct GrammarEntry {
    pub name: String,
    pub language: Language,
    pub node_kinds: Vec<String>,
    pub query_str: Option<String>,
    pub query: Option<Query>,
    // Hold onto the library so memory symbols remain valid in process memory
    _lib_handle: Option<Arc<libloading::Library>>,
}

/// Central Grammar Registry managing static and dynamic language parsers
pub struct GrammarRegistry {
    entries: HashMap<String, Arc<GrammarEntry>>,
    extension_map: HashMap<String, String>, // ext -> language name
}

impl Default for GrammarRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GrammarRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            entries: HashMap::new(),
            extension_map: HashMap::new(),
        };

        // Register default static built-in grammars
        registry.register_builtin(
            "rust",
            vec!["rs".to_string()],
            tree_sitter_rust::LANGUAGE.into(),
            vec![
                "function_item".to_string(),
                "closure_expression".to_string(),
            ],
            Some("(function_item) @function"),
        );

        registry.register_builtin(
            "python",
            vec!["py".to_string()],
            tree_sitter_python::LANGUAGE.into(),
            vec!["function_definition".to_string(), "lambda".to_string()],
            Some("(function_definition) @function"),
        );

        registry
    }

    pub fn register_builtin(
        &mut self,
        name: &str,
        extensions: Vec<String>,
        language: Language,
        node_kinds: Vec<String>,
        query_str: Option<&str>,
    ) {
        let query = query_str.and_then(|q| Query::new(&language, q).ok());
        let entry = Arc::new(GrammarEntry {
            name: name.to_string(),
            language,
            node_kinds,
            query_str: query_str.map(|s| s.to_string()),
            query,
            _lib_handle: None,
        });

        self.entries.insert(name.to_string(), entry);
        for ext in extensions {
            self.extension_map.insert(ext, name.to_string());
        }
    }

    /// Load dynamic grammar from a shared library (.so / .dylib / .dll)
    ///
    /// # Safety
    /// The caller must ensure that `config.library_path` points to a valid shared library
    /// adhering to the Tree-sitter C ABI.
    pub unsafe fn load_dynamic(&mut self, config: &LanguageConfig) -> Result<(), String> {
        let lib_path = config
            .library_path
            .as_ref()
            .ok_or_else(|| "No library_path provided for dynamic grammar".to_string())?;

        if !lib_path.exists() {
            return Err(format!("Shared library path not found: {:?}", lib_path));
        }

        let lib = unsafe {
            libloading::Library::new(lib_path)
                .map_err(|e| format!("Failed to load shared library {:?}: {}", lib_path, e))?
        };

        let symbol_name = config
            .symbol_name
            .clone()
            .unwrap_or_else(|| format!("tree_sitter_{}", config.name));

        let language: Language = unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn() -> *const std::ffi::c_void> = lib
                .get(symbol_name.as_bytes())
                .map_err(|e| format!("Symbol '{}' not found in library: {}", symbol_name, e))?;

            let ptr = func();
            if ptr.is_null() {
                return Err(format!("Symbol '{}' returned null pointer", symbol_name));
            }
            Language::from_raw(ptr.cast())
        };

        let lib_arc = Arc::new(lib);

        let query = config.query.as_ref().and_then(|q_str| {
            Query::new(&language, q_str)
                .map_err(|e| {
                    eprintln!(
                        "Warning: Failed to compile custom query for {}: {}",
                        config.name, e
                    );
                })
                .ok()
        });

        let entry = Arc::new(GrammarEntry {
            name: config.name.clone(),
            language,
            node_kinds: config.node_kinds.clone().unwrap_or_default(),
            query_str: config.query.clone(),
            query,
            _lib_handle: Some(lib_arc),
        });

        self.entries.insert(config.name.clone(), entry);
        for ext in &config.extensions {
            self.extension_map.insert(ext.clone(), config.name.clone());
        }

        Ok(())
    }

    /// Resolve grammar entry by file extension with fallback handling
    pub fn get_by_extension(&self, ext: &str) -> Option<Arc<GrammarEntry>> {
        let lang_name = self.extension_map.get(ext)?;
        self.entries.get(lang_name).cloned()
    }

    /// Load user configurations from TOML
    pub fn apply_config(&mut self, config: RegistryConfig) {
        for lang_cfg in config.languages {
            if lang_cfg.library_path.is_some() {
                unsafe {
                    if let Err(e) = self.load_dynamic(&lang_cfg) {
                        eprintln!("Failed to load dynamic grammar '{}': {}", lang_cfg.name, e);
                    }
                }
            } else if let Some(existing) = self.entries.get(&lang_cfg.name) {
                // Override built-in config if specified
                let node_kinds = lang_cfg
                    .node_kinds
                    .clone()
                    .unwrap_or_else(|| existing.node_kinds.clone());

                let query_str = lang_cfg
                    .query
                    .clone()
                    .or_else(|| existing.query_str.clone());

                let query = if let Some(q_str) = &query_str {
                    Query::new(&existing.language, q_str).ok()
                } else {
                    None
                };

                let entry = Arc::new(GrammarEntry {
                    name: existing.name.clone(),
                    language: existing.language.clone(),
                    node_kinds,
                    query_str,
                    query,
                    _lib_handle: existing._lib_handle.clone(),
                });

                self.entries.insert(lang_cfg.name.clone(), entry);
                for ext in &lang_cfg.extensions {
                    self.extension_map
                        .insert(ext.clone(), lang_cfg.name.clone());
                }
            }
        }
    }
}

/// Test helper to detect enclosing function using AST node kinds
pub fn find_enclosing_function_by_kind(node: Node, kinds: &[String]) -> Option<(usize, usize)> {
    let mut current = Some(node);
    while let Some(n) = current {
        if kinds.iter().any(|k| k == n.kind()) {
            let start = n.start_position().row + 1;
            let end = n.end_position().row + 1;
            return Some((start, end));
        }
        current = n.parent();
    }
    None
}

/// Fallback Strategy Demonstration
pub enum NavigationMode {
    FunctionMode,
    LineMode,
    FileMode,
}

pub fn resolve_navigation_mode(
    file_ext: &str,
    registry: &GrammarRegistry,
) -> (NavigationMode, Option<Arc<GrammarEntry>>) {
    match registry.get_by_extension(file_ext) {
        Some(grammar) => (NavigationMode::FunctionMode, Some(grammar)),
        None => {
            // Graceful fallback to File Mode when no parser is registered
            eprintln!(
                "Notice: No Tree-sitter parser available for extension '.{}'. Falling back to File Mode.",
                file_ext
            );
            (NavigationMode::FileMode, None)
        }
    }
}

fn main() {
    println!("=== Tree-sitter Extensibility Architecture PoC ===");

    let mut registry = GrammarRegistry::new();

    // 1. Test Static Built-ins
    println!("\n--- 1. Static Built-in Grammars ---");
    if let Some(rust_entry) = registry.get_by_extension("rs") {
        println!("Resolved '.rs' -> Language: {}", rust_entry.name);
        println!("Supported node kinds: {:?}", rust_entry.node_kinds);
        println!("Query compiled: {}", rust_entry.query.is_some());
    }

    // 2. Test Fallback Strategy for Unknown Extensions
    println!("\n--- 2. Fallback Mechanism for Unknown File Types ---");
    let (mode, entry) = resolve_navigation_mode("zig", &registry);
    println!(
        "Extension '.zig' resolved mode: {:?}",
        match mode {
            NavigationMode::FunctionMode => "FunctionMode",
            NavigationMode::LineMode => "LineMode",
            NavigationMode::FileMode => "FileMode",
        }
    );
    assert!(entry.is_none());

    // 3. Test TOML Config Override & User Extension Mapping
    println!("\n--- 3. TOML Configuration Integration ---");
    let config_toml = r#"
    [[languages]]
    name = "rust"
    extensions = ["rs", "rs.in"]
    node_kinds = ["function_item", "closure_expression", "macro_definition"]
    query = "(function_item) @function"
    "#;

    let cfg: RegistryConfig = toml::from_str(config_toml).expect("Valid TOML");
    registry.apply_config(cfg);

    if let Some(rust_entry) = registry.get_by_extension("rs.in") {
        println!("Resolved '.rs.in' -> Language: {}", rust_entry.name);
        println!("Updated node kinds: {:?}", rust_entry.node_kinds);
    }

    // 4. Test Parsing Code & AST Node Extraction
    println!("\n--- 4. Code Parsing & AST Traversal ---");
    let rust_code = r#"
    fn hello_world() {
        println!("Hello, Tree-sitter!");
    }
    "#;

    let rust_grammar = registry.get_by_extension("rs").unwrap();
    let mut parser = Parser::new();
    parser.set_language(&rust_grammar.language).unwrap();
    let tree = parser.parse(rust_code, None).unwrap();
    let root = tree.root_node();

    // Find node at row 2, col 8
    let point = tree_sitter::Point { row: 2, column: 8 };
    let node = root.descendant_for_point_range(point, point).unwrap();
    let range = find_enclosing_function_by_kind(node, &rust_grammar.node_kinds);

    println!("Cursor at Point (row 2, col 8) in Rust code:");
    if let Some((start, end)) = range {
        println!("Found enclosing function range: Lines {}-{}", start, end);
    }

    println!("\n=== PoC Executed Successfully ===");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_registration_and_lookup() {
        let registry = GrammarRegistry::new();
        assert!(registry.get_by_extension("rs").is_some());
        assert!(registry.get_by_extension("py").is_some());
        assert!(registry.get_by_extension("unknown").is_none());
    }

    #[test]
    fn test_toml_configuration_override() {
        let mut registry = GrammarRegistry::new();
        let config_toml = r#"
        [[languages]]
        name = "python"
        extensions = ["py", "pyi", "gyp"]
        node_kinds = ["function_definition", "async_function_definition", "lambda"]
        "#;

        let cfg: RegistryConfig = toml::from_str(config_toml).unwrap();
        registry.apply_config(cfg);

        let entry = registry.get_by_extension("pyi").expect("Registered pyi");
        assert_eq!(entry.name, "python");
        assert!(
            entry
                .node_kinds
                .contains(&"async_function_definition".to_string())
        );
    }

    #[test]
    fn test_fallback_navigation_mode() {
        let registry = GrammarRegistry::new();
        let (mode, entry) = resolve_navigation_mode("unknown_ext", &registry);
        assert!(matches!(mode, NavigationMode::FileMode));
        assert!(entry.is_none());
    }
}
