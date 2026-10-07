use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tree_sitter::{Language, Query};

/// Configuration for loading dynamic grammars via config file.
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

/// A loaded grammar entry in memory.
pub struct GrammarEntry {
    pub name: String,
    pub language: Language,
    pub node_kinds: Vec<String>,
    pub query_str: Option<String>,
    query: OnceLock<Option<Query>>,
    pub _lib_handle: Option<Arc<libloading::Library>>,
}

impl GrammarEntry {
    /// The compiled highlight query, if the grammar has a valid one. Compiling is slow
    /// enough to notice at startup, so it happens the first time a language is highlighted.
    pub fn query(&self) -> Option<&Query> {
        self.query
            .get_or_init(|| {
                self.query_str
                    .as_deref()
                    .and_then(|q| Query::new(&self.language, q).ok())
            })
            .as_ref()
    }
}

/// Central Grammar Registry managing static built-in and dynamic shared-object parsers.
pub struct GrammarRegistry {
    entries: HashMap<String, Arc<GrammarEntry>>,
    extension_map: HashMap<String, String>,
}

impl Default for GrammarRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GrammarRegistry {
    /// Create a new registry populated with all static built-in language grammars.
    pub fn new() -> Self {
        let mut registry = Self {
            entries: HashMap::new(),
            extension_map: HashMap::new(),
        };

        // 1. Rust
        registry.register_builtin(
            "rust",
            vec!["rs".to_string()],
            tree_sitter_rust::LANGUAGE.into(),
            vec![
                "function_item".to_string(),
                "closure_expression".to_string(),
            ],
            Some(tree_sitter_rust::HIGHLIGHTS_QUERY),
        );

        // 2. C
        registry.register_builtin(
            "c",
            vec!["c".to_string(), "h".to_string()],
            tree_sitter_c::LANGUAGE.into(),
            vec!["function_definition".to_string()],
            Some(tree_sitter_c::HIGHLIGHT_QUERY),
        );

        // 3. C++
        registry.register_builtin(
            "cpp",
            vec![
                "cpp".to_string(),
                "hpp".to_string(),
                "cc".to_string(),
                "cxx".to_string(),
                "hh".to_string(),
            ],
            tree_sitter_cpp::LANGUAGE.into(),
            vec![
                "function_definition".to_string(),
                "template_declaration".to_string(),
            ],
            Some(tree_sitter_cpp::HIGHLIGHT_QUERY),
        );

        // 4. Go
        registry.register_builtin(
            "go",
            vec!["go".to_string()],
            tree_sitter_go::LANGUAGE.into(),
            vec![
                "function_declaration".to_string(),
                "method_declaration".to_string(),
                "func_literal".to_string(),
            ],
            Some(tree_sitter_go::HIGHLIGHTS_QUERY),
        );

        // 5. Python
        registry.register_builtin(
            "python",
            vec!["py".to_string()],
            tree_sitter_python::LANGUAGE.into(),
            vec!["function_definition".to_string(), "lambda".to_string()],
            Some(tree_sitter_python::HIGHLIGHTS_QUERY),
        );

        // 6. TypeScript / JavaScript
        registry.register_builtin(
            "typescript",
            vec!["ts".to_string(), "js".to_string()],
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            vec![
                "function_declaration".to_string(),
                "function_expression".to_string(),
                "arrow_function".to_string(),
                "method_definition".to_string(),
                "generator_function_declaration".to_string(),
                "generator_function".to_string(),
            ],
            Some(tree_sitter_typescript::HIGHLIGHTS_QUERY),
        );

        // 7. TSX / JSX
        registry.register_builtin(
            "tsx",
            vec!["tsx".to_string(), "jsx".to_string()],
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            vec![
                "function_declaration".to_string(),
                "function_expression".to_string(),
                "arrow_function".to_string(),
                "method_definition".to_string(),
            ],
            Some(tree_sitter_typescript::HIGHLIGHTS_QUERY),
        );

        // 8. Lua
        registry.register_builtin(
            "lua",
            vec!["lua".to_string()],
            tree_sitter_lua::LANGUAGE.into(),
            vec![
                "function_declaration".to_string(),
                "function_definition".to_string(),
            ],
            Some(tree_sitter_lua::HIGHLIGHTS_QUERY),
        );

        registry
    }

    /// Register a static built-in language entry.
    pub fn register_builtin(
        &mut self,
        name: &str,
        extensions: Vec<String>,
        language: Language,
        node_kinds: Vec<String>,
        query_str: Option<&str>,
    ) {
        let entry = Arc::new(GrammarEntry {
            name: name.to_string(),
            language,
            node_kinds,
            query_str: query_str.map(|s| s.to_string()),
            query: OnceLock::new(),
            _lib_handle: None,
        });

        self.entries.insert(name.to_string(), entry);
        for ext in extensions {
            self.extension_map.insert(ext, name.to_string());
        }
    }

    /// Load dynamic grammar from a shared object (`.so` / `.dylib`).
    ///
    /// # Safety
    /// Caller must ensure `config.library_path` points to a valid Tree-sitter C ABI shared object.
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

        let node_kinds = config.node_kinds.clone().unwrap_or_default();

        let entry = Arc::new(GrammarEntry {
            name: config.name.clone(),
            language,
            node_kinds,
            query_str: config.query.clone(),
            query: OnceLock::new(),
            _lib_handle: Some(lib_arc),
        });

        self.entries.insert(config.name.clone(), entry);
        for ext in &config.extensions {
            self.extension_map.insert(ext.clone(), config.name.clone());
        }

        Ok(())
    }

    /// Retrieve grammar entry by file extension.
    pub fn get_by_extension(&self, extension: &str) -> Option<Arc<GrammarEntry>> {
        let clean_ext = extension.trim_start_matches('.');
        let name = self.extension_map.get(clean_ext)?;
        self.entries.get(name).cloned()
    }

    /// Retrieve grammar entry by language name.
    pub fn get_by_name(&self, name: &str) -> Option<Arc<GrammarEntry>> {
        self.entries.get(name).cloned()
    }
}
