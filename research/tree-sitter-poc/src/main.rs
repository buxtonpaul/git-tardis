use tree_sitter::{Language, Parser, Point};

/// Returns the language definition based on file path extension
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

fn run_test_case(
    file_path: &str,
    source_code: &str,
    query_line: usize,
    expected: Option<(usize, usize)>,
) {
    let result = find_enclosing_function_range(file_path, source_code, query_line).unwrap();
    println!(
        "File: {:<8} | Line: {:<2} | Expected: {:<12?} | Got: {:<12?} | Status: {}",
        file_path,
        query_line,
        expected,
        result,
        if result == expected { "\u{2705} PASS" } else { "\u{274C} FAIL" }
    );
    assert_eq!(result, expected);
}

fn main() {
    println!("=== Tree-sitter Enclosing Function Parsing Tests ===");

    // Test 1: Rust
    let rust_code = r#"fn add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum
}

struct Dummy;

fn main() {
    let result = add(2, 3);
    println!("result: {}", result);
}
"#;
    run_test_case("test.rs", rust_code, 1, Some((1, 4))); // on Rust fn signature
    run_test_case("test.rs", rust_code, 2, Some((1, 4))); // inside add
    run_test_case("test.rs", rust_code, 6, None);          // outside fn (struct)
    run_test_case("test.rs", rust_code, 9, Some((8, 11))); // inside main

    // Test 2: Python
    let python_code = r#"def calculate_sum(a, b):
    # Summing up
    result = a + b
    return result

class Math:
    def multiply(self, a, b):
        return a * b

print("Top level code")
"#;
    run_test_case("test.py", python_code, 1, Some((1, 4)));  // signature of calculate_sum
    run_test_case("test.py", python_code, 3, Some((1, 4)));  // body of calculate_sum
    run_test_case("test.py", python_code, 7, Some((7, 8)));  // inside method multiply
    run_test_case("test.py", python_code, 10, None);         // top level print

    // Test 3: Go
    let go_code = r#"package main

import "fmt"

func helper() {
    fmt.Println("helper")
}

func (m *MyType) Solve(input string) error {
    if input == "" {
        return fmt.Errorf("empty")
    }
    return nil
}
"#;
    run_test_case("test.go", go_code, 5, Some((5, 7)));    // helper signature
    run_test_case("test.go", go_code, 6, Some((5, 7)));    // helper body
    run_test_case("test.go", go_code, 9, Some((9, 14)));   // method declaration Solve
    run_test_case("test.go", go_code, 11, Some((9, 14)));  // deep inside method Solve

    // Test 4: TypeScript / TSX
    let ts_code = r#"const arrowFunc = (x: number) => {
    return x * 2;
};

class User {
    private name: string;

    constructor(name: string) {
        this.name = name;
    }

    public getName(): string {
        const uppercase = this.name.toUpperCase();
        return uppercase;
    }
}
"#;
    run_test_case("test.ts", ts_code, 1, Some((1, 3)));    // arrow function
    run_test_case("test.ts", ts_code, 8, Some((8, 10)));   // constructor
    run_test_case("test.ts", ts_code, 12, Some((12, 15))); // getName method signature
    run_test_case("test.ts", ts_code, 13, Some((12, 15))); // getName method body

    // Test 5: Fault-tolerance with syntax errors
    let broken_rust_code = r#"fn broken_func() {
    let x = ; // syntax error!
}
"#;
    let mut parser = Parser::new();
    let r_lang = tree_sitter_rust::LANGUAGE.into();
    parser.set_language(&r_lang).unwrap();
    let broken_tree = parser.parse(broken_rust_code, None).unwrap();
    println!("Broken Rust AST: {}", broken_tree.root_node().to_sexp());

    // Tree-sitter should gracefully still parse broken_func enclosing lines
    run_test_case("test.rs", broken_rust_code, 2, Some((1, 3)));

    println!("\nAll test cases executed and passed successfully!");
}
