use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiffLineType {
    Context,
    Added,
    Modified,
    Deleted,
    HunkHeader,
    DiffHeader,
}

/// Classify a raw diff line string (e.g. when viewing unified diff text directly)
pub fn classify_diff_line(line: &str) -> DiffLineType {
    if line.starts_with("@@") {
        DiffLineType::HunkHeader
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        DiffLineType::DiffHeader
    } else if line.starts_with('+') && !line.starts_with("+++") {
        DiffLineType::Added
    } else if line.starts_with('-') && !line.starts_with("---") {
        DiffLineType::Deleted
    } else {
        DiffLineType::Context
    }
}

/// Parse unified diff hunk header line: `@@ -old_start,old_count +new_start,new_count @@`
pub fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    let start = header.find("@@")? + 2;
    let end = header[start..].find("@@")? + start;
    let range_str = header[start..end].trim();

    let mut old_start = 1;
    let mut old_count = 1;
    let mut new_start = 1;
    let mut new_count = 1;

    for part in range_str.split_whitespace() {
        if let Some(rest) = part.strip_prefix('-') {
            let mut nums = rest.split(',');
            if let Some(s) = nums.next().and_then(|n| n.parse().ok()) {
                old_start = s;
            }
            if let Some(c) = nums.next().and_then(|n| n.parse().ok()) {
                old_count = c;
            } else if part.contains(',') {
                old_count = 0;
            }
        } else if let Some(rest) = part.strip_prefix('+') {
            let mut nums = rest.split(',');
            if let Some(s) = nums.next().and_then(|n| n.parse().ok()) {
                new_start = s;
            }
            if let Some(c) = nums.next().and_then(|n| n.parse().ok()) {
                new_count = c;
            } else if part.contains(',') {
                new_count = 0;
            }
        }
    }

    Some((old_start, old_count, new_start, new_count))
}

/// Parse unified diff text and return line number -> DiffLineType for the new file version
pub fn parse_file_diff_hunks(diff_text: &str) -> HashMap<usize, DiffLineType> {
    let mut highlights = HashMap::new();
    let mut current_new_line = 0;
    let mut deleted_count = 0;
    let mut in_hunk = false;

    for line in diff_text.lines() {
        if line.starts_with("@@") {
            if let Some((_, _, new_start, _)) = parse_hunk_header(line) {
                current_new_line = new_start;
                deleted_count = 0;
                in_hunk = true;
                continue;
            }
        }

        if !in_hunk {
            continue;
        }

        if line.starts_with("diff ") || line.starts_with("index ") {
            in_hunk = false;
            continue;
        }

        if line.starts_with('+') && !line.starts_with("+++") {
            if deleted_count > 0 {
                highlights.insert(current_new_line, DiffLineType::Modified);
                deleted_count -= 1;
            } else {
                highlights.insert(current_new_line, DiffLineType::Added);
            }
            current_new_line += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deleted_count += 1;
        } else if line.starts_with('\\') {
            // Skip "\ No newline at end of file"
            continue;
        } else {
            // Context line (starts with space or empty)
            deleted_count = 0;
            current_new_line += 1;
        }
    }

    highlights
}

/// Helper to map a cursor line index within `code_lines` (which may be a unified diff or regular file content)
/// to the corresponding 1-based target line number in the actual file.
pub fn resolve_file_line_number(code_lines: &[String], cursor_line: usize) -> usize {
    if cursor_line == 0 || code_lines.is_empty() {
        return cursor_line;
    }

    // Check if code_lines appears to be unified diff output
    let is_diff = code_lines
        .iter()
        .any(|l| l.starts_with("@@ ") || l.starts_with("diff --git "));
    if !is_diff {
        return cursor_line;
    }

    let mut current_old_line = 1;
    let mut current_new_line = 1;
    let mut in_hunk = false;

    let target_idx = cursor_line
        .saturating_sub(1)
        .min(code_lines.len().saturating_sub(1));

    for (idx, line) in code_lines.iter().enumerate() {
        if line.starts_with("@@ ") {
            if let Some((old_start, _, new_start, _)) = parse_hunk_header(line) {
                current_old_line = old_start;
                current_new_line = new_start;
                in_hunk = true;
                if idx == target_idx {
                    return current_new_line;
                }
                continue;
            }
        }

        if !in_hunk {
            if idx == target_idx {
                return 1;
            }
            continue;
        }

        if line.starts_with("diff ") || line.starts_with("index ") {
            in_hunk = false;
            if idx == target_idx {
                return current_new_line;
            }
            continue;
        }

        if line.starts_with('-') && !line.starts_with("---") {
            // Deleted line: returns the pre-image line number before deletion
            if idx == target_idx {
                return current_old_line;
            }
            current_old_line += 1;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            // Added line: returns the post-image line number
            if idx == target_idx {
                return current_new_line;
            }
            current_new_line += 1;
        } else if line.starts_with('\\') {
            if idx == target_idx {
                return current_new_line;
            }
        } else {
            // Context line
            if idx == target_idx {
                return current_new_line;
            }
            current_old_line += 1;
            current_new_line += 1;
        }
    }

    current_new_line
}

/// Map a 1-based line number in version A to its corresponding 1-based line number in version B
/// using unified diff text between version A and version B.
pub fn map_line_number(diff_text: &str, old_line: usize) -> usize {
    if old_line == 0 {
        return 1;
    }
    if diff_text.is_empty() {
        return old_line;
    }

    let mut cur_old = 1;
    let mut cur_new = 1;
    let mut in_hunk = false;

    for line in diff_text.lines() {
        if line.starts_with("@@") {
            if let Some((old_start, _, new_start, _)) = parse_hunk_header(line) {
                if !in_hunk {
                    if old_line < old_start {
                        return old_line;
                    }
                } else if old_line < old_start {
                    let delta = old_line.saturating_sub(cur_old);
                    return cur_new + delta;
                }

                cur_old = old_start;
                cur_new = new_start;
                in_hunk = true;
                continue;
            }
        }

        if !in_hunk {
            continue;
        }

        if line.starts_with("diff ") || line.starts_with("index ") {
            if in_hunk && old_line >= cur_old {
                let delta = old_line.saturating_sub(cur_old);
                return cur_new + delta;
            }
            in_hunk = false;
            continue;
        }

        if line.starts_with('-') && !line.starts_with("---") {
            if cur_old == old_line {
                return cur_new;
            }
            cur_old += 1;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            cur_new += 1;
        } else if line.starts_with('\\') {
            continue;
        } else {
            if cur_old == old_line {
                return cur_new;
            }
            cur_old += 1;
            cur_new += 1;
        }
    }

    if in_hunk && old_line >= cur_old {
        let delta = old_line.saturating_sub(cur_old);
        return cur_new + delta;
    }

    old_line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_diff_line() {
        assert_eq!(classify_diff_line("+added line"), DiffLineType::Added);
        assert_eq!(classify_diff_line("-deleted line"), DiffLineType::Deleted);
        assert_eq!(
            classify_diff_line("@@ -1,2 +1,3 @@"),
            DiffLineType::HunkHeader
        );
        assert_eq!(
            classify_diff_line("diff --git a/f b/f"),
            DiffLineType::DiffHeader
        );
        assert_eq!(classify_diff_line(" context line"), DiffLineType::Context);
    }

    #[test]
    fn test_parse_hunk_header() {
        let (os, oc, ns, nc) = parse_hunk_header("@@ -10,3 +12,5 @@ fn test()").unwrap();
        assert_eq!(os, 10);
        assert_eq!(oc, 3);
        assert_eq!(ns, 12);
        assert_eq!(nc, 5);
    }

    #[test]
    fn test_parse_file_diff_hunks_addition_and_modification() {
        let diff = r#"diff --git a/main.rs b/main.rs
index 1234567..89abcde 100644
--- a/main.rs
+++ b/main.rs
@@ -1,3 +1,4 @@
 line 1
-line 2
+line 2 modified
+line 2.5 added
 line 3
"#;

        let highlights = parse_file_diff_hunks(diff);
        assert_eq!(highlights.get(&1), None); // Context
        assert_eq!(highlights.get(&2), Some(&DiffLineType::Modified));
        assert_eq!(highlights.get(&3), Some(&DiffLineType::Added));
        assert_eq!(highlights.get(&4), None); // Context
    }

    #[test]
    fn test_resolve_file_line_number_regular_code() {
        let lines = vec![
            "fn main() {".into(),
            "    println!(\"hi\");".into(),
            "}".into(),
        ];
        assert_eq!(resolve_file_line_number(&lines, 1), 1);
        assert_eq!(resolve_file_line_number(&lines, 2), 2);
        assert_eq!(resolve_file_line_number(&lines, 3), 3);
    }

    #[test]
    fn test_resolve_file_line_number_diff_text() {
        let diff_lines: Vec<String> = vec![
            "diff --git a/main.rs b/main.rs", // 1
            "index 1234567..89abcde 100644",  // 2
            "--- a/main.rs",                  // 3
            "+++ b/main.rs",                  // 4
            "@@ -10,3 +10,4 @@",              // 5
            " line 10",                       // 6 (context line -> new 10, old 10)
            "-deleted line 11",               // 7 (deleted line -> old 11)
            "+added line 11",                 // 8 (added line -> new 11)
            "+added line 12",                 // 9 (added line -> new 12)
            " line 13",                       // 10 (context line -> new 13, old 12)
        ]
        .into_iter()
        .map(String::from)
        .collect();

        // Header lines default to valid line 1
        assert_eq!(resolve_file_line_number(&diff_lines, 1), 1);
        assert_eq!(resolve_file_line_number(&diff_lines, 5), 10);

        // Context line 10
        assert_eq!(resolve_file_line_number(&diff_lines, 6), 10);

        // Deleted line -> old line 11
        assert_eq!(resolve_file_line_number(&diff_lines, 7), 11);

        // Added lines -> new line 11, 12
        assert_eq!(resolve_file_line_number(&diff_lines, 8), 11);
        assert_eq!(resolve_file_line_number(&diff_lines, 9), 12);

        // Context line 13
        assert_eq!(resolve_file_line_number(&diff_lines, 10), 13);
    }

    #[test]
    fn test_map_line_number() {
        let diff = r#"diff --git a/main.rs b/main.rs
--- a/main.rs
+++ b/main.rs
@@ -1,4 +1,6 @@
 line 1
+line 1.5 added
+line 1.6 added
 line 2
-line 3
+line 3 modified
 line 4
"#;

        // line 1 -> line 1
        assert_eq!(map_line_number(diff, 1), 1);
        // line 2 -> line 4 (because 2 lines added before it)
        assert_eq!(map_line_number(diff, 2), 4);
        // line 3 (modified) -> line 5
        assert_eq!(map_line_number(diff, 3), 5);
        // line 4 -> line 6
        assert_eq!(map_line_number(diff, 4), 6);
        // line 10 (after hunk) -> line 12
        assert_eq!(map_line_number(diff, 10), 12);
    }
}
