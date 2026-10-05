/// A text edit for a rename operation: `start_col..end_col` (0-based
/// byte columns) on the 1-based `line` of `file` becomes `new_text`.
#[derive(Debug, Clone)]
pub struct RenameEdit {
    pub file: String,
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub new_text: String,
}

/// `text` with `edits` applied: each replaces its byte range on its 1-based
/// line. The edits must all belong to `text`'s file (callers filter by
/// `file`); an edit whose line is past the end is ignored.
pub fn apply_edits<'a>(text: &str, edits: impl IntoIterator<Item = &'a RenameEdit>) -> String {
    let mut by_line: std::collections::BTreeMap<usize, Vec<&RenameEdit>> =
        std::collections::BTreeMap::new();
    for edit in edits {
        by_line.entry(edit.line).or_default().push(edit);
    }
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let Some(line_edits) = by_line.get_mut(&(index + 1)) else {
            out.push_str(line);
            continue;
        };
        // Right to left, so earlier columns stay valid.
        line_edits.sort_by_key(|e| std::cmp::Reverse(e.start_col));
        let mut line = line.to_string();
        for edit in line_edits.iter() {
            line.replace_range(edit.start_col..edit.end_col, &edit.new_text);
        }
        out.push_str(&line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(line: usize, start_col: usize, end_col: usize, new_text: &str) -> RenameEdit {
        RenameEdit {
            file: "a.spec".to_string(),
            line,
            start_col,
            end_col,
            new_text: new_text.to_string(),
        }
    }

    #[test]
    fn several_edits_on_one_line_keep_their_columns() {
        let text = "uses [ab, ab]\nab\n";
        // Given left to right; applied right to left.
        let edits = [edit(1, 6, 8, "alpha"), edit(1, 10, 12, "alpha")];
        assert_eq!(apply_edits(text, &edits), "uses [alpha, alpha]\nab\n");
    }

    #[test]
    fn byte_columns_after_multibyte_text_land_on_the_identifier() {
        // "é" and "→" are 2 and 3 bytes: the columns are byte offsets.
        let text = "title \"é→\" ab\n";
        let start = text.find("ab").unwrap();
        let edits = [edit(1, start, start + 2, "beta")];
        assert_eq!(apply_edits(text, &edits), "title \"é→\" beta\n");
    }

    #[test]
    fn edits_on_other_lines_leave_the_rest_and_a_missing_newline_alone() {
        let text = "ab\nkeep\nab";
        let edits = [edit(3, 0, 2, "gamma"), edit(9, 0, 2, "never")];
        assert_eq!(apply_edits(text, &edits), "ab\nkeep\ngamma");
    }
}
