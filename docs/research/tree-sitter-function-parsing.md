# Research: Tree-sitter Function Parsing in Rust

This research document details the technical solution for loading and using Tree-sitter in Rust to locate the exact start and end lines of the function/method enclosing a user's cursor line. It provides support for multiple languages dynamically and handles parsing edge cases gracefully.

## Executive Summary

To support the timeline navigation feature of Git-tardis (moving back/forth to commits affecting the "current function"), we must dynamically inspect the abstract syntax tree (AST) of the active file.

This document details:
1. The necessary Rust dependencies and versions.
2. How to map file types to Tree-sitter language grammars.
3. How to walk up the AST from a specific cursor coordinate (`Point`) to find the enclosing function node.
4. How to search down for assigned function declarations (e.g. arrow functions in TypeScript).
5. Lists of grammar node types used for function/method definitions across Rust, Go, Python, and TypeScript.
6. Graceful handling of edge cases (unindented spaces, top-level code, and syntax errors).

---

## 1. Rust Dependencies

The core **`tree-sitter`** library and grammar crates must have aligned ABI versions. In Tree-sitter, the library supports a specific range of parser ABI versions (for example, `tree-sitter = "0.26"` supports ABI version 13 through 15).

### Cargo.toml Configuration:
```toml
[dependencies]
tree-sitter = "0.26"
tree-sitter-rust = "0.24"
tree-sitter-go = "0.23"
tree-sitter-python = "0.23"
tree-sitter-typescript = "0.23"
```

---

## 2. Supported Languages & Grammar Node Types

Different languages have different grammar specifications and name their function-like nodes differently. To detect functions reliably across languages, we match the AST node's `kind()` against known function identifier strings:

| Language | Crate | File Extensions | Node `kind()` Identifier Strings |
| :--- | :--- | :--- | :--- |
| **Rust** | `tree-sitter-rust` | `.rs` | `"function_item"`, `"closure_expression"` |
| **Go** | `tree-sitter-go` | `.go` | `"function_declaration"`, `"method_declaration"`, `"func_literal"` |
| **Python** | `tree-sitter-python` | `.py` | `"function_definition"`, `"lambda"` |
| **TypeScript / JS** | `tree-sitter-typescript` | `.ts`, `.js`, `.tsx`, `.jsx` | `"function_declaration"`, `"function_expression"`, `"arrow_function"`, `"method_definition"`, `"generator_function_declaration"`, `"generator_function"` |

### Rust Mapping function:
```rust
fn is_function_node(kind: &str, lang: &str) -> bool {
    match lang {
        "rust" => matches!(kind, "function_item" | "closure_expression"),
        "python" => matches!(kind, "function_definition" | "lambda"),
        "go" => matches!(kind, "function_declaration" | "method_declaration" | "func_literal"),
        "typescript" | "tsx" | "javascript" => matches!(
            kind,
            "function_declaration"
                | "function_expression"
                | "arrow_function"
                | "method_definition"
                | "generator_function_declaration"
                | "generator_function"
        ),
        _ => false,
    }
}
```

---

## 3. Dynamic Cursor Line Resolution Algorithm

Mapping a 1-based editor line number to the enclosing function node requires robust handling of whitespaces, nesting, and assignments:

### Step 1: Whitespace Normalization
Using raw line starts (`column: 0`) often falls onto leading indentation whitespace. If a function is nested (e.g. a Python method inside a class), this whitespace belongs to the parent class node, not the function. To resolve this:
* Locate the first non-whitespace character on the query line and use its index as the column coordinate for the search `Point`.

### Step 2: Bottom-Up AST Traversal (Enclosing Scope)
Retrieve the smallest node at our search point using `root_node.descendant_for_point_range(point, point)` and walk up its parent chain using `.parent()`.
* If a parent's `.kind()` matches our function list, return its start and end lines immediately.
* This naturally finds the most immediate, nested closure or function first.

### Step 3: Top-Down Declarator Inspection (Signature Line Assignments)
For variable-assigned functions (e.g., `const foo = () => {}` in JS/TS), the cursor might rest on the `const` keyword at the beginning of the line. Walking up from `const` leads to `lexical_declaration` and `program` (skipping `arrow_function` because it resides on a parallel branch of the statement).
* If bottom-up traversal yields `None` but the line is a statement container (e.g. `lexical_declaration`), search down its descendants. If a function/arrow node is found, return its range.

---

## 4. Helper Module Implementation

Below is the verified Rust helper module implementation:

```rust
use tree_sitter::{Language, Parser, Point};

/// Returns the language definition and identifier based on file path extension
fn get_language_for_file(path: &str) -> Option<(Language, &'static str)> {
    let extension = path.split('.').last()?;
    match extension {
        "rs" => Some((tree_sitter_rust::LANGUAGE.into(), "rust")),
        "go" => Some((tree_sitter_go::LANGUAGE.into(), "go")),
        "py" => Some((tree_sitter_python::LANGUAGE.into(), "python")),
        "ts" | "js" => Some((tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "typescript")),
        "tsx" | "jsx" => Some((tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx")),
        _ => None,
    }
}

/// Checks if a node kind represents a function/method in the given language
fn is_function_node(kind: &str, lang: &str) -> bool {
    match lang {
        "rust" => matches!(kind, "function_item" | "closure_expression"),
        "python" => matches!(kind, "function_definition" | "lambda"),
        "go" => matches!(kind, "function_declaration" | "method_declaration" | "func_literal"),
        "typescript" | "tsx" | "javascript" => matches!(
            kind,
            "function_declaration"
                | "function_expression"
                | "arrow_function"
                | "method_definition"
                | "generator_function_declaration"
                | "generator_function"
        ),
        _ => false,
    }
}

/// Main helper: parses a file and returns (start_line, end_line) of the enclosing function.
/// All lines are 1-based.
pub fn find_enclosing_function_range(
    file_path: &str,
    source_code: &str,
    line_number: usize,
) -> Result<Option<(usize, usize)>, String> {
    if line_number == 0 {
        return Err("Line number must be 1-based (greater than 0)".to_string());
    }

    // 1. Detect language
    let (lang, lang_name) = get_language_for_file(file_path)
        .ok_or_else(|| format!("Unsupported file extension for: {}", file_path))?;

    // 2. Parse source code
    let mut parser = Parser::new();
    parser
        .set_language(&lang)
        .map_err(|e| format!("Failed to set language: {:?}", e))?;

    let tree = parser
        .parse(source_code, None)
        .ok_or_else(|| "Failed to parse source code".to_string())?;

    let root_node = tree.root_node();

    // 3. Setup line range
    let row_index = line_number - 1;
    let lines: Vec<&str> = source_code.lines().collect();
    if row_index >= lines.len() {
        return Ok(None);
    }
    let line = lines[row_index];
    let first_non_ws = line.find(|c: char| !c.is_whitespace()).unwrap_or(0);
    let start_point = Point { row: row_index, column: first_non_ws };
    let end_point = Point { row: row_index, column: line.len() };

    // 4a. Walk UP from the first non-whitespace character on the line
    let mut node = root_node.descendant_for_point_range(start_point, start_point);
    while let Some(current) = node {
        if is_function_node(current.kind(), lang_name) {
            let start_line = current.start_position().row + 1;
            let end_line = current.end_position().row + 1;
            return Ok(Some((start_line, end_line)));
        }
        node = current.parent();
    }

    // 4b. If not found walking UP, search DOWN in the line's AST node for assigned functions (e.g., const f = () => {})
    if let Some(line_node) = root_node.descendant_for_point_range(start_point, end_point) {
        let mut stack = vec![line_node];
        while let Some(curr) = stack.pop() {
            if is_function_node(curr.kind(), lang_name) {
                let start_line = curr.start_position().row + 1;
                let end_line = curr.end_position().row + 1;
                return Ok(Some((start_line, end_line)));
            }
            let mut cursor = curr.walk();
            for child in curr.children(&mut cursor) {
                stack.push(child);
            }
        }
    }

    Ok(None)
}
```

---

## 5. Graceful Handling of Edge Cases & Syntax Errors

### Localized Syntax Errors (Balanced Braces)
If there is a localized syntax error (e.g., `let x = ;`), Tree-sitter's fault-tolerant parsing isolates it into an localized `(ERROR)` node inside the function block. Traversal up from the error node works perfectly:
```
AST: (source_file (function_item name: (identifier) body: (block (let_declaration (ERROR)))))
```
* **Result**: Walking up from the `ERROR` node encounters the `block` and successfully resolves to the enclosing `function_item`.

### Severe Syntax Errors (Missing Scope Braces)
If a function block fails to close (e.g. missing matching `}`), Tree-sitter cannot recognize the function construct. It parses the entire block as a flat list of nodes inside an outer `(ERROR)` node:
```
AST: (source_file (ERROR (identifier) (parameters) (let_declaration)))
```
* **Result**: Because there is no valid `function_item` in the tree, the algorithm returns `None`. This is the correct, safe fallback since a non-terminated scope cannot be verified as a valid function boundary.

---

## Citations & Sources
* **Tree-sitter Official Documentation**: [tree-sitter.github.io](https://tree-sitter.github.io/tree-sitter/)
* **Tree-sitter Rust API Reference**: [Docs.rs Tree-sitter](https://docs.rs/tree-sitter/latest/tree_sitter/)
* **Language Grammar Grammars**:
  * [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust)
  * [tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go)
  * [tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python)
  * [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript)
