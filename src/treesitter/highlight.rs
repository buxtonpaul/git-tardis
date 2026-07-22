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

    for line_num in clamped_start..=clamped_end {
        let row = line_num - 1;
        let line_text = lines[row];
        let line_start_byte = line_byte_offsets[row];
        let line_end_byte = line_start_byte + line_text.len();

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
        let relevant: Vec<&ByteCapture> = byte_captures
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
