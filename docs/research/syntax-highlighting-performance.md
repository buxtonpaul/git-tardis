# Research: Syntax Highlighting Rendering Performance in Ratatui

This research document details the technical strategy and performance benchmarks for rendering syntax-highlighted code in Ratatui code viewing panels within Git-tardis.

## Executive Summary

To deliver responsive TUI rendering at 60 FPS without UI jank or frame drops during rapid scrolling and diff navigation, Git-tardis requires a syntax highlighting engine that is both fast and memory efficient.

### Core Findings & Answers:
1. **Engine Selection (`syntect` vs Tree-sitter)**: **Tree-sitter** is the clear winner for Git-tardis. In benchmark testing on a 5,000-line code file in release mode, Tree-sitter highlights the full file in **20.7 ms** (~0.004 ms/line) compared to `syntect`'s **94.6 ms** (~0.019 ms/line) — a **~4.5x to 10x speedup**. Furthermore, Git-tardis already includes Tree-sitter dependencies for function boundary detection, making Tree-sitter AST-aware semantic highlighting zero-overhead in terms of binary architecture.
2. **Ratatui `Text` / `Line` / `Span` Mapping Efficiency**:
   - **Viewport Cropping**: Rather than allocating full-file `Span` object vectors for 5,000+ lines on every frame, Git-tardis tracks parser state line-by-line but only constructs `Line` / `Span` vectors for the visible viewport (e.g. 40 rows). This reduces allocation time from **93.8 ms** down to **8.8 ms** (**10x reduction**).
   - **Span Merging**: Merging adjacent spans with identical `Style` attributes reduces the total number of `Span` objects across the viewport by **60.5%** (from 377 down to 149 spans), significantly reducing render tree traversal and Crossterm buffer diffing costs.
3. **Theme Syncing**:
   - **Neovim RPC Integration**: When running inside or alongside Neovim, Git-tardis queries Neovim's active highlight groups using `nvim_get_hl(0, { name = "Group" })` via RPC. RGB integer values (`fg: 13583321`, `bg: 2039583`) and text attributes (`bold`, `italic`) map directly into Ratatui `Color::Rgb(r, g, b)` and `Modifier` structs.
   - **Terminal ANSI Fallback**: When running standalone, Git-tardis falls back to `Color::Indexed(...)` and standard 16-color ANSI rules, allowing terminal emulators (Alacritty, Kitty, WezTerm) to theme the TUI automatically.

---

## 1. Syntax Engine Comparison: `syntect` vs `tree-sitter-highlight`

### Architectural Comparison

| Dimension | `syntect` | `tree-sitter-highlight` |
| :--- | :--- | :--- |
| **Parsing Model** | Regex-based state machine (`Oniguruma` / `fancy-regex`) operating line by line. | Fast C/Rust GLR AST parser walking nodes via compiled grammar queries (`highlights.scm`). |
| **AST Awareness** | Minimal / Heuristic. Cannot easily distinguish variable types from function calls or parameter scope. | Full AST awareness. Differentiates types, fields, parameters, macros, closure scopes, and function definitions. |
| **Performance (5k lines, Release)** | **94.62 ms** | **20.74 ms** (**4.5x faster**) |
| **Performance (1k lines, Debug)** | **127.80 ms** | **20.24 ms** (**6.3x faster**) |
| **Binary Footprint** | Adds ~2–5 MB binary dump for Sublime Text `.sublime-syntax` definitions. | ~100–300 KB per compiled C grammar parser crate. |
| **Existing Git-tardis Fit** | Requires new dependency and regex engine. | Already integrated in Git-tardis for function range queries (Issue #3). |

### Benchmark Results (Measured on Apple Silicon / Rust 1.90)

#### Cold Parse & Full Highlight Throughput
- **100 lines**:
  - `syntect`: `9.38 ms` (release) / `24.50 ms` (debug)
  - `tree-sitter`: `0.62 ms` (release) / `2.36 ms` (debug)
- **1,000 lines**:
  - `syntect`: `30.15 ms` (release) / `127.80 ms` (debug)
  - `tree-sitter`: `4.32 ms` (release) / `20.24 ms` (debug)
- **5,000 lines**:
  - `syntect`: `94.63 ms` (release) / `600.35 ms` (debug)
  - `tree-sitter`: `20.75 ms` (release) / `101.42 ms` (debug)

---

## 2. Efficient Mapping into Ratatui `Text` / `Line` / `Span`

To achieve 60 FPS in Ratatui, rendering must not perform heavy string allocations or construct unused widgets outside the visible bounds.

### Optimization Strategy 1: Viewport Cropping (Virtual Scrolling)
In a code view panel with 5,000 lines and a 40-line visible viewport:
- **Naive approach**: Parse and allocate `Vec<Span>` for all 5,000 lines every frame. Benchmark time: **93.87 ms** per render.
- **Viewport Cropping**: Compute syntax tokens / AST once, and only allocate `Line` / `Span` structures for rows `viewport_top..viewport_top + viewport_height`. Benchmark time: **8.85 ms** per render.

### Optimization Strategy 2: Span Merging
Tree-sitter and regex highlighters often yield multiple small adjacent tokens with identical styling (e.g. whitespace, variable names, contiguous punctuation).

```
Unmerged Spans:  [Span("pub", Keyword), Span(" ", Keyword), Span("fn", Keyword)]
Merged Span:     [Span("pub fn", Keyword)]
```

- **Unmerged Spans in Viewport**: 377 spans across 40 lines.
- **Merged Spans in Viewport**: 149 spans across 40 lines.
- **Reduction**: **60.5% reduction** in total `Span` allocations and Ratatui buffer diff operations.

### Span Merging Rust Algorithm
```rust
pub fn merge_adjacent_spans<'a>(spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
    let mut merged: Vec<Span<'a>> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(last) = merged.last_mut() {
            if last.style == span.style {
                // Combine text content when styles match exactly
                let combined = format!("{}{}", last.content, span.content);
                *last = Span::styled(combined, last.style);
                continue;
            }
        }
        merged.push(span);
    }
    merged
}
```

---

## 3. Theme Syncing: Terminal ANSI & Neovim RPC

### Neovim RPC Highlight Group Resolution
When running inside or launched by Neovim, Git-tardis can synchronize its colors with the host editor's active theme (e.g. Tokyonight, Catppuccin, Gruvbox) by querying `nvim_get_hl`:

#### Neovim RPC Call
```lua
-- Fetch active highlight group colors (returns integer RGB hex and flags)
vim.api.nvim_get_hl(0, { link = false })
```

#### JSON Payload Example
```json
{
  "Keyword": { "fg": 13583321, "bold": true },
  "Function": { "fg": 8959226 },
  "String": { "fg": 10082980 },
  "Comment": { "fg": 8421504, "italic": true },
  "Type": { "fg": 16752762 },
  "Normal": { "fg": 14540253, "bg": 2039583 }
}
```

#### Rust Highlight Group Parser & Conversion
```rust
use ratatui::style::{Color, Modifier, Style};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct ThemeMap {
    pub keyword: Style,
    pub function: Style,
    pub string: Style,
    pub comment: Style,
    pub type_def: Style,
    pub normal: Style,
}

impl ThemeMap {
    pub fn from_nvim_hl_json(val: &Value) -> Self {
        let mut theme = ThemeMap::default();
        if let Some(hl) = val.get("Keyword") { theme.keyword = parse_hl(hl); }
        if let Some(hl) = val.get("Function") { theme.function = parse_hl(hl); }
        if let Some(hl) = val.get("String") { theme.string = parse_hl(hl); }
        if let Some(hl) = val.get("Comment") { theme.comment = parse_hl(hl); }
        if let Some(hl) = val.get("Type") { theme.type_def = parse_hl(hl); }
        if let Some(hl) = val.get("Normal") { theme.normal = parse_hl(hl); }
        theme
    }
}

fn parse_hl(val: &Value) -> Style {
    let mut style = Style::default();
    if let Some(fg) = val.get("fg").and_then(|v| v.as_u64()) {
        let r = ((fg >> 16) & 0xFF) as u8;
        let g = ((fg >> 8) & 0xFF) as u8;
        let b = (fg & 0xFF) as u8;
        style = style.fg(Color::Rgb(r, g, b));
    }
    if let Some(bg) = val.get("bg").and_then(|v| v.as_u64()) {
        let r = ((bg >> 16) & 0xFF) as u8;
        let g = ((bg >> 8) & 0xFF) as u8;
        let b = (bg & 0xFF) as u8;
        style = style.bg(Color::Rgb(r, g, b));
    }
    if val.get("bold").and_then(|v| v.as_bool()).unwrap_or(false) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if val.get("italic").and_then(|v| v.as_bool()).unwrap_or(false) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    style
}
```

### Tree-sitter Capture to Neovim Highlight Mapping Table

| Tree-sitter Query Capture | Standard Neovim Highlight Group | Default ANSI Fallback |
| :--- | :--- | :--- |
| `@keyword`, `@keyword.function` | `Keyword` | `Color::Magenta` |
| `@function`, `@method` | `Function` | `Color::Blue` |
| `@string` | `String` | `Color::Green` |
| `@comment` | `Comment` | `Color::DarkGray` |
| `@type`, `@type.builtin` | `Type` | `Color::Yellow` |
| `@variable`, `@property` | `Identifier` | `Color::White` |
| `@constant`, `@number` | `Constant` | `Color::Cyan` |

---

## 4. PoC Code & Verification

The proof of concept crate located in `research/syntax-highlighting-poc/` validates all benchmarks and mappings:

- **Location**: `research/syntax-highlighting-poc/`
- **Execution**: `cargo run --release --manifest-path research/syntax-highlighting-poc/Cargo.toml`
- **Results**: Verified Tree-sitter execution speed (~20.7ms for 5,000 lines), 60.5% span merging reduction, and Neovim RPC `nvim_get_hl` integer hex to `ratatui::style::Style` mapping.
