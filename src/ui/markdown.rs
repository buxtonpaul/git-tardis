use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// Check if a path or filename represents a Markdown file
pub fn is_markdown_file(path: Option<&str>) -> bool {
    let path = match path {
        Some(p) => p,
        None => return false,
    };
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "md" | "markdown" | "mdown" | "mkdn" | "mkd")
}

/// Formatter state for tracking multi-line structures (e.g., code blocks)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownFormatterState {
    pub in_code_block: bool,
}

impl MarkdownFormatterState {
    pub fn new() -> Self {
        Self {
            in_code_block: false,
        }
    }

    /// Update multi-line state for a line that is not being rendered (e.g. scrolled out of view).
    pub fn advance(&mut self, line: &str) {
        if line.trim().starts_with("```") {
            self.in_code_block = !self.in_code_block;
        }
    }
}

/// Render a single line of Markdown text into a list of styled Ratatui Spans
pub fn render_markdown_line(
    line: &str,
    state: &mut MarkdownFormatterState,
    base_style: Style,
) -> Vec<Span<'static>> {
    let trimmed = line.trim();

    // 1. Code Block Fence check
    if trimmed.starts_with("```") {
        state.in_code_block = !state.in_code_block;
        let fence_style = base_style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
        return vec![Span::styled(line.to_string(), fence_style)];
    }

    // 2. Lines inside code block
    if state.in_code_block {
        let code_style = base_style.fg(Color::Green);
        return vec![Span::styled(line.to_string(), code_style)];
    }

    // 3. Horizontal Rule
    if is_horizontal_rule(trimmed) {
        let hr_style = base_style.fg(Color::DarkGray);
        return vec![Span::styled(
            "────────────────────────".to_string(),
            hr_style,
        )];
    }

    // 4. Headers
    if let Some((level, content)) = parse_header(trimmed) {
        let (color, modifier) = match level {
            1 => (Color::Cyan, Modifier::BOLD | Modifier::UNDERLINED),
            2 => (Color::Yellow, Modifier::BOLD),
            3 => (Color::Green, Modifier::BOLD),
            4 => (Color::Magenta, Modifier::BOLD),
            5 => (Color::Blue, Modifier::BOLD),
            _ => (Color::DarkGray, Modifier::BOLD),
        };

        let prefix_style = base_style.fg(color).add_modifier(modifier);
        let header_style = base_style.fg(color).add_modifier(modifier);

        let hashes = "#".repeat(level);
        let mut spans = vec![Span::styled(format!("{} ", hashes), prefix_style)];
        spans.extend(parse_inline_spans(content, header_style));
        return spans;
    }

    // 5. Blockquotes
    if trimmed.starts_with('>') {
        let quote_content = trimmed.strip_prefix('>').unwrap_or("").trim_start();
        let prefix_style = base_style.fg(Color::LightYellow);
        let quote_style = base_style
            .fg(Color::LightYellow)
            .add_modifier(Modifier::ITALIC);

        let mut spans = vec![Span::styled("│ ".to_string(), prefix_style)];
        spans.extend(parse_inline_spans(quote_content, quote_style));
        return spans;
    }

    // 6. Checkbox lists (- [ ] or - [x])
    if let Some((is_checked, content)) = parse_checkbox(trimmed) {
        let (box_str, box_style) = if is_checked {
            (
                "[✓] ",
                base_style.fg(Color::Green).add_modifier(Modifier::BOLD),
            )
        } else {
            ("[ ] ", base_style.fg(Color::DarkGray))
        };

        let mut spans = vec![Span::styled(box_str.to_string(), box_style)];
        spans.extend(parse_inline_spans(content, base_style));
        return spans;
    }

    // 7. Bullet / Numbered lists
    if let Some((prefix, content)) = parse_list_item(trimmed) {
        let bullet_style = base_style.fg(Color::Yellow);
        let mut spans = vec![Span::styled(format!("{} ", prefix), bullet_style)];
        spans.extend(parse_inline_spans(content, base_style));
        return spans;
    }

    // 8. Normal text line with inline formatting
    parse_inline_spans(line, base_style)
}

fn is_horizontal_rule(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() < 3 {
        return false;
    }
    let first = chars[0];
    (first == '-' || first == '*' || first == '_') && chars.iter().all(|&c| c == first)
}

fn parse_header(s: &str) -> Option<(usize, &str)> {
    if !s.starts_with('#') {
        return None;
    }
    let level = s.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&level) {
        let rest = s[level..].trim_start();
        Some((level, rest))
    } else {
        None
    }
}

fn parse_checkbox(s: &str) -> Option<(bool, &str)> {
    let rest = s
        .strip_prefix("- ")
        .or_else(|| s.strip_prefix("* "))
        .or_else(|| s.strip_prefix("+ "))?;

    if let Some(r) = rest.strip_prefix("[ ] ") {
        Some((false, r))
    } else if let Some(r) = rest.strip_prefix("[x] ") {
        Some((true, r))
    } else {
        rest.strip_prefix("[X] ").map(|r| (true, r))
    }
}

fn parse_list_item(s: &str) -> Option<(&str, &str)> {
    if let Some(rest) = s.strip_prefix("- ") {
        Some(("•", rest))
    } else if let Some(rest) = s.strip_prefix("* ") {
        Some(("•", rest))
    } else if let Some(rest) = s.strip_prefix("+ ") {
        Some(("•", rest))
    } else {
        if let Some(dot_idx) = s.find(". ") {
            let num_part = &s[..dot_idx];
            if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                let rest = &s[dot_idx + 2..];
                return Some((&s[..=dot_idx], rest));
            }
        }
        None
    }
}

/// Parse inline formatting (code, bold, italic, strikethrough, links) into Ratatui spans
pub fn parse_inline_spans(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code `` `code` ``
        if chars[i] == '`' {
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), base_style));
                current.clear();
            }
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != '`' {
                i += 1;
            }
            let code_str: String = chars[start..i].iter().collect();
            if i < chars.len() {
                i += 1; // skip closing `
            }
            let code_style = base_style.fg(Color::Yellow);
            spans.push(Span::styled(code_str, code_style));
            continue;
        }

        // Strikethrough `~~text~~`
        if i + 1 < chars.len() && chars[i] == '~' && chars[i + 1] == '~' {
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), base_style));
                current.clear();
            }
            i += 2;
            let start = i;
            while i + 1 < chars.len() && !(chars[i] == '~' && chars[i + 1] == '~') {
                i += 1;
            }
            let strike_str: String = chars[start..i].iter().collect();
            if i + 1 < chars.len() {
                i += 2; // skip closing ~~
            }
            let strike_style = base_style.add_modifier(Modifier::CROSSED_OUT);
            spans.push(Span::styled(strike_str, strike_style));
            continue;
        }

        // Bold `**text**` or `__text__`
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let delim = chars[i];
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), base_style));
                current.clear();
            }
            i += 2;
            let start = i;
            while i + 1 < chars.len() && !(chars[i] == delim && chars[i + 1] == delim) {
                i += 1;
            }
            let bold_str: String = chars[start..i].iter().collect();
            if i + 1 < chars.len() {
                i += 2; // skip closing ** or __
            }
            let bold_style = base_style.add_modifier(Modifier::BOLD);
            spans.extend(parse_inline_spans(&bold_str, bold_style));
            continue;
        }

        // Italic `*text*` or `_text_`
        if (chars[i] == '*' || chars[i] == '_') && (i == 0 || chars[i - 1] != chars[i]) {
            let delim = chars[i];
            if i + 1 < chars.len() && chars[i + 1] == delim {
                current.push(chars[i]);
                i += 1;
                continue;
            }
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), base_style));
                current.clear();
            }
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != delim {
                i += 1;
            }
            let italic_str: String = chars[start..i].iter().collect();
            if i < chars.len() {
                i += 1; // skip closing * or _
            }
            let italic_style = base_style.add_modifier(Modifier::ITALIC);
            spans.extend(parse_inline_spans(&italic_str, italic_style));
            continue;
        }

        // Links `[label](url)`
        if chars[i] == '[' {
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), base_style));
                current.clear();
            }
            let start_label = i + 1;
            if let Some(close_bracket) = text[start_label..].find(']') {
                let close_bracket_idx = start_label + close_bracket;
                if close_bracket_idx + 1 < chars.len() && chars[close_bracket_idx + 1] == '(' {
                    let start_url = close_bracket_idx + 2;
                    if let Some(close_paren) = text[start_url..].find(')') {
                        let close_paren_idx = start_url + close_paren;
                        let label: String = chars[start_label..close_bracket_idx].iter().collect();
                        let url: String = chars[start_url..close_paren_idx].iter().collect();

                        let link_style = base_style
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::UNDERLINED);
                        let url_style = base_style.fg(Color::DarkGray);

                        spans.push(Span::styled(label, link_style));
                        spans.push(Span::styled(format!(" ({})", url), url_style));

                        i = close_paren_idx + 1;
                        continue;
                    }
                }
            }
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, base_style));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Some("README.md")));
        assert!(is_markdown_file(Some("docs/guide.markdown")));
        assert!(is_markdown_file(Some("file.mdown")));
        assert!(!is_markdown_file(Some("main.rs")));
        assert!(!is_markdown_file(None));
    }

    #[test]
    fn test_render_markdown_header_and_lists() {
        let mut state = MarkdownFormatterState::new();
        let base = Style::default();

        let header_spans = render_markdown_line("# Title", &mut state, base);
        assert_eq!(header_spans[0].content, "# ");
        assert_eq!(header_spans[1].content, "Title");

        let list_spans = render_markdown_line("- Item 1", &mut state, base);
        assert_eq!(list_spans[0].content, "• ");
        assert_eq!(list_spans[1].content, "Item 1");

        let task_spans = render_markdown_line("- [x] Done", &mut state, base);
        assert_eq!(task_spans[0].content, "[✓] ");
        assert_eq!(task_spans[1].content, "Done");
    }

    #[test]
    fn test_render_markdown_code_block() {
        let mut state = MarkdownFormatterState::new();
        let base = Style::default();

        let fence1 = render_markdown_line("```rust", &mut state, base);
        assert!(state.in_code_block);
        assert_eq!(fence1[0].content, "```rust");

        let code_line = render_markdown_line("fn main() {}", &mut state, base);
        assert_eq!(code_line[0].content, "fn main() {}");

        let fence2 = render_markdown_line("```", &mut state, base);
        assert!(!state.in_code_block);
        assert_eq!(fence2[0].content, "```");
    }

    #[test]
    fn test_parse_inline_spans() {
        let base = Style::default();
        let spans = parse_inline_spans(
            "Hello **world** and `code` with [Link](https://example.com)",
            base,
        );

        assert_eq!(spans[0].content, "Hello ");
        assert_eq!(spans[1].content, "world");
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[2].content, " and ");
        assert_eq!(spans[3].content, "code");
        assert_eq!(spans[3].style.fg, Some(Color::Yellow));
        assert_eq!(spans[4].content, " with ");
        assert_eq!(spans[5].content, "Link");
        assert_eq!(spans[6].content, " (https://example.com)");
    }
}
