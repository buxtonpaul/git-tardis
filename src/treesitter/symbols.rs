use tree_sitter::Parser;

use super::registry::GrammarRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolItem {
    pub name: String,
    pub kind: String,
    pub line_number: usize,
}

/// Extract symbols from source code using Tree-sitter or regex/keyword fallback
pub fn extract_symbols(
    registry: &GrammarRegistry,
    file_path: &str,
    source_code: &str,
) -> Vec<SymbolItem> {
    let mut symbols = Vec::new();

    let extension = file_path.rsplit('.').next().unwrap_or("");
    if let Some(entry) = registry.get_by_extension(extension) {
        let mut parser = Parser::new();
        if parser.set_language(&entry.language).is_ok() {
            if let Some(tree) = parser.parse(source_code, None) {
                let root_node = tree.root_node();
                let mut stack = vec![root_node];

                while let Some(node) = stack.pop() {
                    let kind_str = node.kind();
                    if is_symbol_node_kind(kind_str) {
                        let line_number = node.start_position().row + 1;
                        let name = extract_node_name(node, source_code)
                            .unwrap_or_else(|| format!("<anonymous {}>", kind_str));
                        let friendly_kind = normalize_kind_name(kind_str);

                        symbols.push(SymbolItem {
                            name,
                            kind: friendly_kind.to_string(),
                            line_number,
                        });
                    }

                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        stack.push(child);
                    }
                }
            }
        }
    }

    if symbols.is_empty() {
        symbols = fallback_extract_symbols(source_code);
    } else {
        symbols.sort_by_key(|s| s.line_number);
        symbols.dedup_by(|a, b| a.line_number == b.line_number && a.name == b.name);
    }

    symbols
}

fn is_symbol_node_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "function_declaration"
            | "function_definition"
            | "method_definition"
            | "struct_item"
            | "class_declaration"
            | "class_definition"
            | "trait_item"
            | "enum_item"
            | "type_item"
            | "type_alias"
            | "interface_declaration"
            | "const_item"
            | "impl_item"
    )
}

fn normalize_kind_name(kind: &str) -> &str {
    if kind.contains("function") || kind.contains("method") {
        "fn"
    } else if kind.contains("struct") {
        "struct"
    } else if kind.contains("class") {
        "class"
    } else if kind.contains("trait") || kind.contains("interface") {
        "trait"
    } else if kind.contains("enum") {
        "enum"
    } else if kind.contains("type") {
        "type"
    } else if kind.contains("const") {
        "const"
    } else if kind.contains("impl") {
        "impl"
    } else {
        "symbol"
    }
}

fn extract_node_name(node: tree_sitter::Node, source_code: &str) -> Option<String> {
    if let Some(child) = node.child_by_field_name("name") {
        if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
            return Some(text.trim().to_string());
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "type_identifier" | "field_identifier" | "property_identifier" | "name"
        ) {
            if let Ok(text) = child.utf8_text(source_code.as_bytes()) {
                return Some(text.trim().to_string());
            }
        }
    }

    None
}

fn fallback_extract_symbols(source_code: &str) -> Vec<SymbolItem> {
    let mut symbols = Vec::new();
    let keywords = [
        ("fn ", "fn"),
        ("def ", "fn"),
        ("function ", "fn"),
        ("struct ", "struct"),
        ("class ", "class"),
        ("trait ", "trait"),
        ("enum ", "enum"),
        ("type ", "type"),
        ("interface ", "interface"),
        ("const ", "const"),
        ("impl ", "impl"),
    ];

    for (idx, line) in source_code.lines().enumerate() {
        let trimmed = line.trim();
        let clean = if let Some(rest) = trimmed.strip_prefix("pub ") {
            rest.trim_start()
        } else if let Some(rest) = trimmed.strip_prefix("pub(crate) ") {
            rest.trim_start()
        } else if let Some(rest) = trimmed.strip_prefix("export ") {
            rest.trim_start()
        } else if let Some(rest) = trimmed.strip_prefix("async ") {
            rest.trim_start()
        } else {
            trimmed
        };

        for (kw, kind) in &keywords {
            if let Some(stripped) = clean.strip_prefix(kw) {
                let rest = stripped.trim_start();
                let name = rest
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("")
                    .trim();

                if !name.is_empty() {
                    symbols.push(SymbolItem {
                        name: name.to_string(),
                        kind: kind.to_string(),
                        line_number: idx + 1,
                    });
                    break;
                }
            }
        }
    }

    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_extract_symbols() {
        let code = r#"
pub fn calculate_total() -> u32 { 10 }
struct User { name: String }
pub enum Role { Admin, User }
"#;

        let registry = GrammarRegistry::new();
        let symbols = extract_symbols(&registry, "test.rs", code);

        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "calculate_total");
        assert_eq!(symbols[0].kind, "fn");
        assert_eq!(symbols[0].line_number, 2);

        assert_eq!(symbols[1].name, "User");
        assert_eq!(symbols[1].kind, "struct");
        assert_eq!(symbols[1].line_number, 3);

        assert_eq!(symbols[2].name, "Role");
        assert_eq!(symbols[2].kind, "enum");
        assert_eq!(symbols[2].line_number, 4);
    }
}
