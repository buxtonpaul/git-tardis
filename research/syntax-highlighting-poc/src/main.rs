use std::time::Instant;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::easy::HighlightLines;
use ratatui::style::{Color, Modifier, Style as RatatuiStyle};
use ratatui::text::{Line, Span};
use serde_json::json;

fn main() {
    println!("=== Syntax Highlighting Rendering Performance Benchmarks ===");
    
    // Sample Rust code snippet (replicated to test scaling)
    let sample_code = r#"
pub struct SyntaxEngine {
    theme: Theme,
    cache: std::collections::HashMap<usize, Vec<Span<'static>>>,
}

impl SyntaxEngine {
    pub fn new(theme_name: &str) -> Self {
        let theme = Theme::load(theme_name);
        SyntaxEngine {
            theme,
            cache: std::collections::HashMap::new(),
        }
    }

    pub fn render_line<'a>(&self, line_num: usize, content: &'a str) -> Line<'a> {
        let mut spans = Vec::new();
        if content.is_empty() {
            return Line::from("");
        }
        // Tokenize and style content
        for word in content.split_whitespace() {
            let style = match word {
                "pub" | "struct" | "impl" | "fn" | "let" | "mut" | "return" | "match" => {
                    RatatuiStyle::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                }
                "Self" | "SyntaxEngine" | "Theme" | "Line" | "Span" | "Vec" | "Color" => {
                    RatatuiStyle::default().fg(Color::Yellow)
                }
                _ => RatatuiStyle::default().fg(Color::White),
            };
            spans.push(Span::styled(format!("{} ", word), style));
        }
        Line::from(spans)
    }
}
"#;

    // Scale up sample code to 1000 lines and 5000 lines
    let code_100_lines = sample_code.repeat(3);
    let code_1000_lines = sample_code.repeat(30);
    let code_5000_lines = sample_code.repeat(150);

    println!("\n--- Benchmark 1: Syntect vs Tree-sitter-highlight (Cold Parse & Highlight) ---");
    
    benchmark_syntect("100 lines", &code_100_lines);
    benchmark_syntect("1,000 lines", &code_1000_lines);
    benchmark_syntect("5,000 lines", &code_5000_lines);

    benchmark_treesitter("100 lines", &code_100_lines);
    benchmark_treesitter("1,000 lines", &code_1000_lines);
    benchmark_treesitter("5,000 lines", &code_5000_lines);

    println!("\n--- Benchmark 2: Viewport Cropping & Span Merging Efficiency ---");
    test_viewport_rendering(&code_5000_lines);

    println!("\n--- Benchmark 3: Neovim RPC Highlight Group Theme Mapping ---");
    test_neovim_theme_mapping();
}

fn benchmark_syntect(label: &str, code: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let theme = &ts.themes["base16-ocean.dark"];

    let start = Instant::now();
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut total_spans = 0;

    for line in code.lines() {
        let ranges: Vec<(SyntectStyle, &str)> = highlighter.highlight_line(line, &ps).unwrap();
        total_spans += ranges.len();
    }
    let duration = start.elapsed();
    println!("Syntect [{}]: {:?} (generated {} spans)", label, duration, total_spans);
}

fn benchmark_treesitter(label: &str, code: &str) {
    let mut highlighter = Highlighter::new();
    let mut config = HighlightConfiguration::new(
        tree_sitter_rust::language(),
        "rust",
        tree_sitter_rust::HIGHLIGHTS_QUERY,
        "",
        "",
    ).unwrap();

    let recognized_names = [
        "keyword", "function", "type", "string", "comment", "variable", "number", "operator"
    ];
    config.configure(&recognized_names);

    let start = Instant::now();
    let highlights = highlighter.highlight(&config, code.as_bytes(), None, |_| None).unwrap();

    let mut total_events = 0;
    for event in highlights {
        if let Ok(_ev) = event {
            total_events += 1;
        }
    }
    let duration = start.elapsed();
    println!("Tree-sitter [{}]: {:?} (processed {} events)", label, duration, total_events);
}

fn test_viewport_rendering(code: &str) {
    let lines: Vec<&str> = code.lines().collect();
    let total_lines = lines.len();
    let viewport_height = 40; // typical TUI visible window
    let start_row = 500; // user scrolled down to line 500

    println!("Total code lines: {}, Rendering Viewport lines {}-{}", total_lines, start_row, start_row + viewport_height);

    // Strategy A: Naive full-file highlight then slice viewport
    let start_a = Instant::now();
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension("rs").unwrap();
    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut full_highlighted: Vec<Line> = Vec::with_capacity(total_lines);
    for line in lines.iter() {
        let ranges = highlighter.highlight_line(line, &ps).unwrap();
        let spans: Vec<Span> = ranges.into_iter().map(|(style, text)| {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            Span::styled(text.to_string(), RatatuiStyle::default().fg(fg))
        }).collect();
        full_highlighted.push(Line::from(spans));
    }
    let visible_a = &full_highlighted[start_row..start_row + viewport_height];
    let duration_a = start_a.elapsed();
    println!("Strategy A (Full File Highlight & Slice): {:?} (rendered {} lines)", duration_a, visible_a.len());

    // Strategy B: Viewport-only streaming highlight (for Syntect line-by-line streaming)
    let start_b = Instant::now();
    let mut highlighter_b = HighlightLines::new(syntax, theme);
    // Parse up to start_row to maintain state, but only allocate Spans for visible lines
    for line in &lines[0..start_row] {
        let _ = highlighter_b.highlight_line(line, &ps).unwrap();
    }
    let mut visible_b: Vec<Line> = Vec::with_capacity(viewport_height);
    for line in &lines[start_row..start_row + viewport_height] {
        let ranges = highlighter_b.highlight_line(line, &ps).unwrap();
        let spans: Vec<Span> = ranges.into_iter().map(|(style, text)| {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            Span::styled(text.to_string(), RatatuiStyle::default().fg(fg))
        }).collect();
        visible_b.push(Line::from(spans));
    }
    let duration_b = start_b.elapsed();
    println!("Strategy B (State tracking + Viewport-only Span Allocation): {:?} (rendered {} lines)", duration_b, visible_b.len());

    // Strategy C: Span Merging Optimization
    // Merging consecutive spans with identical Style reduces Ratatui render tree node count
    let unmerged_spans_count: usize = visible_b.iter().map(|l| l.spans.len()).sum();
    
    let merged_lines: Vec<Line> = visible_b.into_iter().map(|line| {
        let mut merged: Vec<Span> = Vec::new();
        for span in line.spans {
            if let Some(last) = merged.last_mut() {
                if last.style == span.style {
                    // Combine text content
                    let combined = format!("{}{}", last.content, span.content);
                    *last = Span::styled(combined, last.style);
                    continue;
                }
            }
            merged.push(span);
        }
        Line::from(merged)
    }).collect();
    let merged_spans_count: usize = merged_lines.iter().map(|l| l.spans.len()).sum();

    println!("Span Merging: Reduced span count from {} to {} spans across viewport ({:.1}% reduction)",
        unmerged_spans_count, merged_spans_count,
        (1.0 - (merged_spans_count as f64 / unmerged_spans_count as f64)) * 100.0);
}

fn test_neovim_theme_mapping() {
    // Simulated JSON payload from Neovim RPC: nvim_get_hl(0, {})
    let nvim_hl_response = json!({
        "Keyword": { "fg": 13583321, "bold": true },      // #CF76D9
        "Function": { "fg": 8959226 },                    // #88BBFA
        "String": { "fg": 10082980 },                    // #99E2A4
        "Comment": { "fg": 8421504, "italic": true },     // #808080
        "Type": { "fg": 16752762 },                       // #FF9D7A
        "Normal": { "fg": 14540253, "bg": 2039583 }       // #DDDDFD / #1F1F1F
    });

    println!("Simulated Neovim RPC `nvim_get_hl` response received.");

    struct NvimHl {
        fg: Option<u32>,
        bg: Option<u32>,
        bold: bool,
        italic: bool,
    }

    fn parse_hl(val: &serde_json::Value) -> NvimHl {
        NvimHl {
            fg: val.get("fg").and_then(|v| v.as_u64()).map(|v| v as u32),
            bg: val.get("bg").and_then(|v| v.as_u64()).map(|v| v as u32),
            bold: val.get("bold").and_then(|v| v.as_bool()).unwrap_or(false),
            italic: val.get("italic").and_then(|v| v.as_bool()).unwrap_or(false),
        }
    }

    fn hl_to_ratatui_style(hl: NvimHl) -> RatatuiStyle {
        let mut style = RatatuiStyle::default();
        if let Some(fg) = hl.fg {
            let r = ((fg >> 16) & 0xFF) as u8;
            let g = ((fg >> 8) & 0xFF) as u8;
            let b = (fg & 0xFF) as u8;
            style = style.fg(Color::Rgb(r, g, b));
        }
        if let Some(bg) = hl.bg {
            let r = ((bg >> 16) & 0xFF) as u8;
            let g = ((bg >> 8) & 0xFF) as u8;
            let b = (bg & 0xFF) as u8;
            style = style.bg(Color::Rgb(r, g, b));
        }
        if hl.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if hl.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        style
    }

    let groups = ["Keyword", "Function", "String", "Comment", "Type", "Normal"];
    for group in groups {
        if let Some(hl_val) = nvim_hl_response.get(group) {
            let hl = parse_hl(hl_val);
            let ratatui_style = hl_to_ratatui_style(hl);
            println!("  Nvim Highlight Group `{}` => Ratatui Style: {:?}", group, ratatui_style);
        }
    }
}
