use git_tardis::treesitter::{
    find_enclosing_function_range, highlight_viewport, GrammarRegistry, LanguageConfig,
};

#[test]
fn test_grammar_registry_builtins() {
    let registry = GrammarRegistry::new();

    let rust_grammar = registry.get_by_extension("rs");
    assert!(rust_grammar.is_some());
    assert_eq!(rust_grammar.unwrap().name, "rust");

    let py_grammar = registry.get_by_extension(".py");
    assert!(py_grammar.is_some());
    assert_eq!(py_grammar.unwrap().name, "python");

    let go_grammar = registry.get_by_extension("go");
    assert!(go_grammar.is_some());
    assert_eq!(go_grammar.unwrap().name, "go");

    let ts_grammar = registry.get_by_extension("ts");
    assert!(ts_grammar.is_some());
    assert_eq!(ts_grammar.unwrap().name, "typescript");

    let lua_grammar = registry.get_by_extension("lua");
    assert!(lua_grammar.is_some());
    assert_eq!(lua_grammar.unwrap().name, "lua");

    let c_grammar = registry.get_by_extension("c");
    assert!(c_grammar.is_some());

    let cpp_grammar = registry.get_by_extension("cpp");
    assert!(cpp_grammar.is_some());
}

#[test]
fn test_dynamic_loader_fallback_behavior() {
    let mut registry = GrammarRegistry::new();

    let config = LanguageConfig {
        name: "nonexistent_lang".to_string(),
        extensions: vec!["nonexistent".to_string()],
        library_path: Some(std::path::PathBuf::from("/invalid/path/to/grammar.so")),
        symbol_name: Some("tree_sitter_nonexistent".to_string()),
        node_kinds: None,
        query: None,
    };

    // Safety: unsafe call tested for non-existent library graceful error handling
    let res = unsafe { registry.load_dynamic(&config) };
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Shared library path not found"));
}

#[test]
fn test_rust_function_scope() {
    let registry = GrammarRegistry::new();
    let code = r#"
fn add(a: i32, b: i32) -> i32 {
    let sum = a + b;
    sum
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;

    // Line 3: inside `add`
    let res_add = find_enclosing_function_range(&registry, "main.rs", code, 3).unwrap();
    assert_eq!(res_add, Some((2, 5)));

    // Line 8: inside `multiply`
    let res_mult = find_enclosing_function_range(&registry, "main.rs", code, 8).unwrap();
    assert_eq!(res_mult, Some((7, 9)));

    // Line 6: outside any function
    let res_outside = find_enclosing_function_range(&registry, "main.rs", code, 6).unwrap();
    assert_eq!(res_outside, None);
}

#[test]
fn test_python_function_scope() {
    let registry = GrammarRegistry::new();
    let code = r#"
def calculate_area(width, height):
    area = width * height
    return area

def greet(name):
    print(f"Hello {name}")
"#;

    let res = find_enclosing_function_range(&registry, "app.py", code, 3).unwrap();
    assert_eq!(res, Some((2, 4)));

    let res_greet = find_enclosing_function_range(&registry, "app.py", code, 7).unwrap();
    assert_eq!(res_greet, Some((6, 7)));
}

#[test]
fn test_typescript_declarator_fallback() {
    let registry = GrammarRegistry::new();
    let code = r#"
const addNumbers = (a: number, b: number): number => {
    return a + b;
};

function normalFunction() {
    console.log("hello");
}
"#;

    // Cursor on line 2 (const statement header)
    let res_arrow = find_enclosing_function_range(&registry, "app.ts", code, 2).unwrap();
    assert_eq!(res_arrow, Some((2, 4)));

    // Line 7
    let res_func = find_enclosing_function_range(&registry, "app.ts", code, 7).unwrap();
    assert_eq!(res_func, Some((6, 8)));
}

#[test]
fn test_go_function_scope() {
    let registry = GrammarRegistry::new();
    let code = r#"
package main

import "fmt"

func ProcessItem(id int) string {
    msg := fmt.Sprintf("Item %d", id)
    return msg
}
"#;

    let res = find_enclosing_function_range(&registry, "main.go", code, 7).unwrap();
    assert_eq!(res, Some((6, 9)));
}

#[test]
fn test_lua_function_scope() {
    let registry = GrammarRegistry::new();
    let code = r#"
local function calculate(x)
    local y = x * 2
    return y
end
"#;

    let res = find_enclosing_function_range(&registry, "script.lua", code, 3).unwrap();
    assert_eq!(res, Some((2, 5)));
}

#[test]
fn test_syntax_fault_tolerance() {
    let registry = GrammarRegistry::new();
    // Invalid code containing localized syntax error
    let invalid_code = r#"
fn valid_func() {
    let x = 10;
    let y = ; // syntax error here!
    let z = x + y;
}
"#;

    // Even with syntax error inside `valid_func`, Tree-sitter isolates error into (ERROR) node
    let res = find_enclosing_function_range(&registry, "test.rs", invalid_code, 4).unwrap();
    assert_eq!(res, Some((2, 6)));
}

#[test]
fn test_viewport_highlight_and_span_merging() {
    let registry = GrammarRegistry::new();
    let rust_entry = registry.get_by_name("rust").unwrap();

    let code = r#"fn compute(x: i32) -> i32 {
    let result = x + 10;
    result
}"#;

    let highlighted = highlight_viewport(&rust_entry, code, 1, 3);
    assert_eq!(highlighted.len(), 3);
    assert_eq!(highlighted[0].line_number, 1);

    // Line 1 should contain keyword "fn" span
    let contains_keyword = highlighted[0]
        .spans
        .iter()
        .any(|s| s.capture_name == "keyword");
    assert!(contains_keyword);
}
