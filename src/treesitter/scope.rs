use tree_sitter::{Parser, Point};

use super::registry::GrammarRegistry;

/// Find the 1-based (start_line, end_line) of the enclosing function surrounding `line_number`.
pub fn find_enclosing_function_range(
    registry: &GrammarRegistry,
    file_path: &str,
    source_code: &str,
    line_number: usize,
) -> Result<Option<(usize, usize)>, String> {
    if line_number == 0 {
        return Err("Line number must be 1-based (greater than 0)".to_string());
    }

    let extension = match file_path.rsplit('.').next() {
        Some(ext) => ext,
        None => return Ok(None),
    };

    let entry = match registry.get_by_extension(extension) {
        Some(e) => e,
        None => return Ok(None),
    };

    let mut parser = Parser::new();
    if parser.set_language(&entry.language).is_err() {
        return Ok(None);
    }

    let tree = match parser.parse(source_code, None) {
        Some(t) => t,
        None => return Ok(None),
    };

    let root_node = tree.root_node();
    let row_index = line_number - 1;
    let lines: Vec<&str> = source_code.lines().collect();

    if row_index >= lines.len() {
        return Ok(None);
    }

    let line = lines[row_index];
    let first_non_ws = line.find(|c: char| !c.is_whitespace()).unwrap_or(0);

    let start_point = Point {
        row: row_index,
        column: first_non_ws,
    };
    let end_point = Point {
        row: row_index,
        column: line.len(),
    };

    // 1. Bottom-up AST traversal starting at first non-whitespace character
    let mut node = root_node.descendant_for_point_range(start_point, start_point);
    while let Some(current) = node {
        if is_target_kind(current.kind(), &entry.node_kinds) {
            let start_line = current.start_position().row + 1;
            let end_line = current.end_position().row + 1;
            return Ok(Some((start_line, end_line)));
        }
        node = current.parent();
    }

    // 2. Top-down declarator fallback search on the line's descendant node
    if let Some(line_node) = root_node.descendant_for_point_range(start_point, end_point) {
        if line_node != root_node
            && line_node.kind() != "source_file"
            && line_node.kind() != "program"
            && line_node.kind() != "translation_unit"
        {
            let mut stack = vec![line_node];
            while let Some(curr) = stack.pop() {
                if is_target_kind(curr.kind(), &entry.node_kinds) {
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
    }

    Ok(None)
}

fn is_target_kind(kind: &str, target_kinds: &[String]) -> bool {
    target_kinds.iter().any(|k| k == kind)
}
