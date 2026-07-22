# Research: Tree-sitter Grammar Extensibility Architecture

This research document evaluates architectural options for extending Tree-sitter language grammar support in **Git-tardis**. It analyzes static compilation vs. dynamic shared library loading (`libloading` / `.so` / `.dylib`), defines user configuration schemas (TOML and Neovim Lua), and details fallback mechanisms when language parsers are unavailable or fail to load.

---

## Executive Summary

Git-tardis relies on Tree-sitter for function-level timeline navigation (Function Mode) and syntax-highlighted side-by-side diff rendering. Currently, core language grammars (Rust, C, C++, Go, Python, TypeScript) are statically compiled into the Rust binary.

To allow users to add custom language grammars, remap file extensions, override AST function node kinds, or dynamically load external grammar binaries without recompiling Git-tardis, we propose a **Hybrid Grammar Architecture**:

1. **Static Built-in Core**: Include high-frequency grammars statically in the binary for zero-configuration, out-of-the-box performance and safety.
2. **Dynamic Plugin Loader**: Load user-provided shared libraries (`.so`/`.dylib`/`.dll`) using `libloading` via standard C ABI symbols (`tree_sitter_<lang>`).
3. **Flexible User Configuration**: Support TOML configuration (`~/.config/git-tardis/config.toml`) and Neovim Lua `setup()` overrides for node kind arrays, file extension mappings, and custom S-expression queries (`.scm`).
4. **Graceful Degradation**: Fall back from **Function Mode** to **File Mode** or **Line Mode** navigation whenever a language parser is missing or fails to load, ensuring zero application crashes.

---

## 1. Static vs. Dynamic Grammar Loading Evaluation

### Trade-off Matrix

| Metric / Dimension | Static Compilation (`tree-sitter-lang`) | Dynamic Loading (`libloading` / `.so` / `.dylib`) |
| :--- | :--- | :--- |
| **Startup Overhead** | Zero runtime cost; pre-linked symbols. | Minor `dlopen` overhead per custom language. |
| **Binary Size** | ~200KB – 1MB per added language grammar. | Small core binary (~few MBs); grammars loaded on demand. |
| **User Ergonomics** | Works immediately out-of-the-box with zero setup. | Requires downloading or compiling C parser shared libraries. |
| **Extensibility** | Recompilation of `git-tardis` required for new languages. | Fully dynamic; users drop `.so` in config dir. |
| **Memory Safety** | 100% Rust memory safety guarantees. | `unsafe` boundary; requires holding `libloading::Library` handle in memory. |
| **ABI Version Stability**| Tied strictly to crate build dependencies. | Requires C ABI compatibility with host `tree-sitter` C runtime. |

### C ABI Compatibility & Memory Safety
Tree-sitter language grammars export a single C function signature:
```c
const TSLanguage *tree_sitter_<langname>(void);
```

In Rust, converting this pointer to `tree_sitter::Language` is performed via `Language::from_raw(ptr)`:
```rust
let func: libloading::Symbol<unsafe extern "C" fn() -> *const std::ffi::c_void> = 
    lib.get(symbol_name.as_bytes())?;
let ptr = func();
let language = unsafe { Language::from_raw(ptr.cast()) };
```

> **Crucial Safety Requirement:** The `tree_sitter::Language` struct holds raw pointers into the memory segment allocated by `libloading::Library`. If the `Library` handle is dropped while `Language` or `Parser` is in use, calling Tree-sitter API methods will result in a **Segmentation Fault**. Therefore, the `GrammarRegistry` must store `Arc<libloading::Library>` alongside the `Language` instance.

### Hybrid Architecture Recommendation
Git-tardis should adopt a **Hybrid Model**:
- **Static Core**: Rust, C, C++, Go, Python, TypeScript, JavaScript, Lua, TOML.
- **Dynamic Extensibility**: Allow users to configure dynamic libraries for languages like Zig, Elixir, Haskell, OCaml, Ruby, etc.

---

## 2. User Configuration Format (TOML & Neovim Lua)

### TOML Configuration Schema (`~/.config/git-tardis/config.toml`)

Users can define custom language extensions, override node kind matchers, or specify dynamic shared library paths:

```toml
# Remap or override existing built-in grammars
[[languages]]
name = "rust"
extensions = ["rs", "rs.in"]
node_kinds = ["function_item", "closure_expression", "macro_definition"]
query = "(function_item) @function"

# Register external dynamic grammar
[[languages]]
name = "zig"
extensions = ["zig"]
library_path = "~/.config/git-tardis/grammars/zig.dylib"
symbol_name = "tree_sitter_zig" # Optional, defaults to tree_sitter_<name>
node_kinds = ["FnProto", "function_declaration"]
query = "(FnProto) @function"
```

### Neovim Lua Plugin Configuration (`git-tardis.setup`)

In `git-tardis.nvim`, the configuration can be passed directly through `setup()`:

```lua
require("git-tardis").setup({
  languages = {
    rust = {
      extensions = { "rs", "rs.in" },
      node_kinds = { "function_item", "closure_expression", "macro_definition" },
    },
    zig = {
      extensions = { "zig" },
      library_path = vim.fn.expand("~/.config/git-tardis/grammars/zig.so"),
      node_kinds = { "FnProto", "function_declaration" },
      query = "(FnProto) @function",
    },
  },
})
```

---

## 3. AST Matching: Node Kinds vs S-Expression Queries

Git-tardis supports two mechanisms for locating enclosing functions:

### 1. AST Node Kind String Matching (Fast Bottom-Up Traversal)
- **Mechanism**: Inspect `.kind()` of parent nodes starting from cursor location.
- **Performance**: $O(d)$ where $d$ is AST tree depth (~5-15 nodes max). Zero allocation.
- **Use Case**: Default strategy for function-level navigation.

```rust
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
```

### 2. Tree-sitter S-Expression Queries (`tree_sitter::Query`)
- **Mechanism**: Compile `.scm` query strings (e.g. `(function_item) @function`) and run `QueryCursor`.
- **Performance**: Slower than parent traversal for single-point lookup, but highly expressive for complex language features (e.g., matching specific decorator patterns or nested struct methods).
- **Note**: `tree_sitter::Query` does not implement `Clone`, so query strings are stored and compiled once per grammar entry.

---

## 4. Fallback Mechanisms & Fault Tolerance

To guarantee stability, Git-tardis uses a 3-tier fallback strategy:

```
[User Action: Navigate Timeline in Function Mode]
                 │
                 ▼
     Has registered parser for .ext?
         ├── YES ──► Parse AST ──► Found Function? ──► [Jump in Function Mode]
         │                              │
         │                              └── NO ──────► [Fall back to Line/File Mode]
         │
         └── NO ──► Log UI Notice ──────────────────► [Fall back to File Mode]
```

### Fallback Hierarchy:
1. **Dynamic Shared Library Failure**:
   If a user's `.so` / `.dylib` fails to load (e.g., file not found, bad C symbol, ABI incompatibility), Git-tardis logs a non-fatal warning in the TUI status bar and falls back to a built-in grammar if available, or switches to **File Mode**.
2. **Invalid Custom Query**:
   If a user provides an invalid `.scm` query string, `Query::new()` compilation fails. Git-tardis ignores the query and falls back to `node_kinds` AST traversal.
3. **Missing Language Parser**:
   If no parser exists for a given file extension, Git-tardis automatically degrades from **Function Mode** to **File Mode** or **Line Mode** navigation rather than displaying an error modal or crashing.

---

## 5. Verification & PoC Implementation

A working proof-of-concept crate has been implemented in `research/tree-sitter-extensibility-poc`:

- **Configuration Parser**: Deserializes TOML language settings and merges user overrides with static built-ins.
- **Grammar Registry**: Stores static and dynamic grammars and maintains shared library handles via `Arc<libloading::Library>`.
- **Unit Tests**: Verified built-in lookups, TOML override application, and graceful fallback to `FileMode` for unknown file extensions (`cargo test` passing).

```
running 3 tests
test tests::test_fallback_navigation_mode ... ok
test tests::test_builtin_registration_and_lookup ... ok
test tests::test_toml_configuration_override ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

---

## 6. Recommendations for Architecture Specification

1. **Adopt Hybrid Model**: Bundle common grammars in binary; support dynamic shared library loading for user-defined languages.
2. **Store Library Handles Safely**: Wrap `libloading::Library` in `Arc` within `GrammarRegistry` entries to prevent premature library unloading.
3. **Unified TOML/Lua Config**: Use identical key structures (`extensions`, `node_kinds`, `library_path`, `query`) across Neovim Lua setup and TOML config files.
4. **Non-blocking Fallbacks**: Treat parser absence or syntax errors as soft mode transitions (Function Mode $\rightarrow$ File Mode) rather than fatal errors.
