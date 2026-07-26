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
}
