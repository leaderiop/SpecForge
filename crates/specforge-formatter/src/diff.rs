/// A unified diff for a single file.
#[derive(Debug, Clone)]
pub struct FormatDiff {
    pub file_path: String,
    pub diff_text: String,
    pub insertions: usize,
    pub deletions: usize,
}

/// Generate a unified diff between original and formatted content.
pub fn unified_diff(file_path: &str, original: &str, formatted: &str) -> FormatDiff {
    if original == formatted {
        return FormatDiff {
            file_path: file_path.to_string(),
            diff_text: String::new(),
            insertions: 0,
            deletions: 0,
        };
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let fmt_lines: Vec<&str> = formatted.lines().collect();

    let mut diff_text = String::new();
    diff_text.push_str(&format!("--- {file_path}\n"));
    diff_text.push_str(&format!("+++ {file_path}\n"));

    let mut insertions = 0;
    let mut deletions = 0;

    // Simple line-by-line diff with context
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < orig_lines.len() || j < fmt_lines.len() {
        if i < orig_lines.len() && j < fmt_lines.len() && orig_lines[i] == fmt_lines[j] {
            i += 1;
            j += 1;
            continue;
        }

        // Found a difference — collect the hunk
        let hunk_start_i = i;
        let hunk_start_j = j;

        // Find where lines sync back up
        while i < orig_lines.len() || j < fmt_lines.len() {
            // Try to find a sync point
            if i < orig_lines.len() && j < fmt_lines.len() && orig_lines[i] == fmt_lines[j] {
                break;
            }
            // Advance the longer side, or both
            if i < orig_lines.len()
                && (j >= fmt_lines.len()
                    || !fmt_lines[j..].contains(&orig_lines[i]))
            {
                i += 1;
            } else if j < fmt_lines.len()
                && (i >= orig_lines.len()
                    || !orig_lines[i..].iter().any(|l| *l == fmt_lines[j]))
            {
                j += 1;
            } else {
                // Both could match later — advance both for simple algorithm
                i += 1;
                j += 1;
            }
        }

        let removed = &orig_lines[hunk_start_i..i];
        let added = &fmt_lines[hunk_start_j..j];

        deletions += removed.len();
        insertions += added.len();

        hunks.push(DiffHunk {
            orig_start: hunk_start_i + 1, // 1-indexed for unified diff
            orig_count: removed.len(),
            new_start: hunk_start_j + 1,
            new_count: added.len(),
            removed: removed.iter().map(|s| s.to_string()).collect(),
            added: added.iter().map(|s| s.to_string()).collect(),
            context_before: if hunk_start_i > 0 {
                orig_lines[hunk_start_i.saturating_sub(3)..hunk_start_i]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            },
            context_after: if i < orig_lines.len() {
                orig_lines[i..orig_lines.len().min(i + 3)]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                Vec::new()
            },
        });
    }

    // Format hunks
    for hunk in &hunks {
        let ctx_before = hunk.context_before.len();
        let ctx_after = hunk.context_after.len();

        let orig_start = hunk.orig_start.saturating_sub(ctx_before);
        let orig_count = ctx_before + hunk.orig_count + ctx_after;
        let new_start = hunk.new_start.saturating_sub(ctx_before);
        let new_count = ctx_before + hunk.new_count + ctx_after;

        diff_text.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            orig_start, orig_count, new_start, new_count
        ));

        for line in &hunk.context_before {
            diff_text.push_str(&format!(" {line}\n"));
        }
        for line in &hunk.removed {
            diff_text.push_str(&format!("-{line}\n"));
        }
        for line in &hunk.added {
            diff_text.push_str(&format!("+{line}\n"));
        }
        for line in &hunk.context_after {
            diff_text.push_str(&format!(" {line}\n"));
        }
    }

    FormatDiff {
        file_path: file_path.to_string(),
        diff_text,
        insertions,
        deletions,
    }
}

struct DiffHunk {
    orig_start: usize,
    orig_count: usize,
    new_start: usize,
    new_count: usize,
    removed: Vec<String>,
    added: Vec<String>,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(behavior = "show_formatting_diff", verify = "diff output uses unified format")]
    #[test]
    fn test_unified_diff_format() {
        let diff = unified_diff(
            "spec/test.spec",
            "  behavior foo \"Foo\" {\n  contract \"a\"\n}\n",
            "behavior foo \"Foo\" {\n  contract \"a\"\n}\n",
        );
        assert!(diff.diff_text.contains("--- spec/test.spec"));
        assert!(diff.diff_text.contains("+++ spec/test.spec"));
        assert!(diff.diff_text.contains("@@"));
    }

    #[specforge_test_macros::test(behavior = "show_formatting_diff", verify = "unchanged files produce no diff output")]
    #[test]
    fn test_diff_unchanged_files_empty() {
        let diff = unified_diff("test.spec", "hello\n", "hello\n");
        assert!(diff.diff_text.is_empty());
        assert_eq!(diff.insertions, 0);
        assert_eq!(diff.deletions, 0);
    }

    #[specforge_test_macros::test(behavior = "show_formatting_diff", verify = "diff output uses unified format")]
    #[test]
    fn test_diff_counts_insertions_deletions() {
        let diff = unified_diff(
            "test.spec",
            "  line1\n  line2\n",
            "line1\nline2\n",
        );
        assert!(diff.insertions > 0 || diff.deletions > 0);
    }
}
