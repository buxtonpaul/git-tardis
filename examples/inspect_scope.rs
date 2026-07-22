use std::env;
use std::fs;

use git_tardis::treesitter::{find_enclosing_function_range, highlight_viewport, GrammarRegistry};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run --example inspect_scope <file_path> <line_number>");
        println!("Example: cargo run --example inspect_scope src/lib.rs 2");
        return;
    }

    let file_path = &args[1];
    let line_number: usize = args[2]
        .parse()
        .expect("Line number must be a valid 1-based integer");

    let source_code = match fs::read_to_string(file_path) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error reading file {}: {}", file_path, err);
            std::process::exit(1);
        }
    };

    let registry = GrammarRegistry::new();

    println!("--- 1. Grammar Lookup ---");
    let ext = file_path.rsplit('.').next().unwrap_or("");
    match registry.get_by_extension(ext) {
        Some(entry) => println!("Matched language grammar: '{}'", entry.name),
        None => println!(
            "No matching tree-sitter grammar registered for extension '.{}'",
            ext
        ),
    }

    println!("\n--- 2. Enclosing Function Scope Query ---");
    match find_enclosing_function_range(&registry, file_path, &source_code, line_number) {
        Ok(Some((start, end))) => {
            println!(
                "Line {} is enclosed in function spanning lines {}-{}",
                line_number, start, end
            );
        }
        Ok(None) => println!("Line {} is outside any function scope.", line_number),
        Err(e) => eprintln!("Error finding function scope: {}", e),
    }

    println!(
        "\n--- 3. Viewport Syntax Highlighting (Lines {}-{}) ---",
        line_number.saturating_sub(2).max(1),
        line_number + 2
    );
    if let Some(entry) = registry.get_by_extension(ext) {
        let start = line_number.saturating_sub(2).max(1);
        let end = line_number + 2;
        let highlighted = highlight_viewport(&entry, &source_code, start, end);
        for line in highlighted {
            print!("{:>4} | ", line.line_number);
            for span in line.spans {
                print!("[{}] {}", span.capture_name, span.text);
            }
            println!();
        }
    }
}
