use ratatui::style::{Color, Style};
use tree_sitter::{Parser, QueryCursor, StreamingIterator};

use super::registry::GrammarEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub text: String,
    pub capture_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightLine {
    pub line_number: usize,
    pub spans: Vec<HighlightSpan>,
}

#[derive(Debug, Clone)]
struct ByteCapture {
    start_byte: usize,
    end_byte: usize,
    capture_name: String,
}

/// Highlight lines in `source_code` within 1-based `start_line..=end_line` range.
pub fn highlight_viewport(
    entry: &GrammarEntry,
    source_code: &str,
    start_line: usize,
    end_line: usize,
) -> Vec<HighlightLine> {
    let lines: Vec<&str> = source_code.lines().collect();
    if lines.is_empty() || start_line == 0 || start_line > lines.len() {
        return Vec::new();
    }

    let clamped_start = start_line.max(1);
    let clamped_end = end_line.min(lines.len());
    if clamped_start > clamped_end {
        return Vec::new();
    }

    // 1. Gather capture intervals if query is available
    let mut byte_captures = Vec::new();
    if let Some(query) = &entry.query {
        let mut parser = Parser::new();
        if parser.set_language(&entry.language).is_ok() {
            if let Some(tree) = parser.parse(source_code, None) {
                let mut cursor = QueryCursor::new();
                let mut matches = cursor.matches(query, tree.root_node(), source_code.as_bytes());

                while let Some(m) = matches.next() {
                    for c in m.captures {
                        let name = query.capture_names()[c.index as usize].to_string();
                        let node = c.node;
                        byte_captures.push(ByteCapture {
                            start_byte: node.start_byte(),
                            end_byte: node.end_byte(),
                            capture_name: name,
                        });
                    }
                }
            }
        }
    }

    byte_captures.sort_by_key(|c| c.start_byte);

    // 2. Compute line start byte offsets in source_code
    let mut line_byte_offsets = Vec::with_capacity(lines.len());
    let mut current_offset = 0;
    for l in &lines {
        line_byte_offsets.push(current_offset);
        current_offset += l.len() + 1; // +1 for \n
    }

    // 3. Generate spans per line in viewport
    let mut highlighted_lines = Vec::with_capacity(clamped_end - clamped_start + 1);

    // Captures are sorted by start byte, so the ones touching a line sit in a window that only
    // moves forward: `first_live` skips captures that ended before the line, and the upper bound
    // is the first capture starting at or after the line end.
    let mut first_live = 0;

    for line_num in clamped_start..=clamped_end {
        let row = line_num - 1;
        let line_text = lines[row];
        let line_start_byte = line_byte_offsets[row];
        let line_end_byte = line_start_byte + line_text.len();

        while first_live < byte_captures.len()
            && byte_captures[first_live].end_byte <= line_start_byte
        {
            first_live += 1;
        }

        if line_text.is_empty() {
            highlighted_lines.push(HighlightLine {
                line_number: line_num,
                spans: vec![HighlightSpan {
                    text: String::new(),
                    capture_name: "normal".to_string(),
                }],
            });
            continue;
        }

        // Filter captures relevant to this line
        let window_end = byte_captures.partition_point(|c| c.start_byte < line_end_byte);
        let relevant: Vec<&ByteCapture> = byte_captures[first_live.min(window_end)..window_end]
            .iter()
            .filter(|c| c.start_byte < line_end_byte && c.end_byte > line_start_byte)
            .collect();

        let mut unmerged_spans = Vec::new();
        let mut curr_byte = line_start_byte;

        while curr_byte < line_end_byte {
            if let Some(cap) = relevant
                .iter()
                .find(|c| c.start_byte <= curr_byte && c.end_byte > curr_byte)
            {
                let seg_end = cap.end_byte.min(line_end_byte);
                let text = source_code[curr_byte..seg_end].to_string();
                unmerged_spans.push(HighlightSpan {
                    text,
                    capture_name: cap.capture_name.clone(),
                });
                curr_byte = seg_end;
            } else {
                let next_cap_start = relevant
                    .iter()
                    .filter(|c| c.start_byte > curr_byte)
                    .map(|c| c.start_byte)
                    .min()
                    .unwrap_or(line_end_byte)
                    .min(line_end_byte);

                let text = source_code[curr_byte..next_cap_start].to_string();
                unmerged_spans.push(HighlightSpan {
                    text,
                    capture_name: "normal".to_string(),
                });
                curr_byte = next_cap_start;
            }
        }

        // 4. Merge adjacent spans sharing identical capture_name
        let merged_spans = merge_adjacent_spans(unmerged_spans);

        highlighted_lines.push(HighlightLine {
            line_number: line_num,
            spans: merged_spans,
        });
    }

    highlighted_lines
}

pub fn merge_adjacent_spans(spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
    let mut merged: Vec<HighlightSpan> = Vec::new();

    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if last.capture_name == span.capture_name {
                last.text.push_str(&span.text);
                continue;
            }
        }
        merged.push(span);
    }

    merged
}

/// Map Tree-sitter capture names to standard Ratatui ANSI color styles.
pub fn capture_name_to_style(capture: &str) -> Style {
    let color = match capture {
        c if c.starts_with("keyword") || c == "repeat" || c == "conditional" => Color::Magenta,
        c if c.starts_with("function") || c.starts_with("method") => Color::Blue,
        c if c.starts_with("type") || c == "structure" || c == "class" => Color::Yellow,
        c if c.starts_with("string") || c == "char" => Color::Green,
        c if c.starts_with("comment") => Color::DarkGray,
        c if c.starts_with("number")
            || c.starts_with("float")
            || c.starts_with("boolean")
            || c.starts_with("constant") =>
        {
            Color::Red
        }
        c if c.starts_with("variable") || c.starts_with("property") || c == "field" => Color::Cyan,
        c if c.starts_with("operator") => Color::LightCyan,
        c if c == "attribute" || c == "macro" => Color::LightYellow,
        c if c.starts_with("punctuation") => Color::Gray,
        _ => Color::Reset,
    };

    if color == Color::Reset {
        Style::default()
    } else {
        Style::default().fg(color)
    }
}
