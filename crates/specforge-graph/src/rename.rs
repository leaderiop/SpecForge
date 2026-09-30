use crate::Graph;
use specforge_common::SourceSpan;

/// A text edit for a rename operation.
#[derive(Debug, Clone)]
pub struct RenameEdit {
    pub file: String,
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub new_text: String,
}

/// Validates that the entity can be renamed and returns its token range.
pub fn prepare_rename(graph: &Graph, entity_id: &str) -> Option<SourceSpan> {
    graph.node(entity_id).map(|n| n.source_span.clone())
}

/// The edits that rename `old_id` to `new_id`: every whole-word occurrence
/// of `old_id` inside its declaration and inside each entity that
/// references it, one edit per occurrence (`start_col`/`end_col` are byte
/// columns on the 1-based `line`). `text_of` supplies a file's text; a file
/// without text contributes no edits. `None` when `old_id` is unknown or
/// `new_id` is taken.
pub fn identifier_edits(
    graph: &Graph,
    old_id: &str,
    new_id: &str,
    text_of: impl Fn(&str) -> Option<String>,
) -> Option<Vec<RenameEdit>> {
    let declaration = graph.node(old_id)?;
    if graph.node(new_id).is_some() {
        return None;
    }

    // The declaration's span and each referencing entity's span.
    let mut spans = vec![&declaration.source_span];
    for edge in graph.edges() {
        if edge.target == old_id
            && let Some(source) = graph.node(edge.source.as_str())
            && !spans.contains(&&source.source_span)
        {
            spans.push(&source.source_span);
        }
    }

    let mut edits = Vec::new();
    for span in spans {
        let Some(text) = text_of(span.file.as_str()) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let line_no = index + 1;
            if line_no < span.start_line || line_no > span.end_line {
                continue;
            }
            for (start, end) in word_occurrences(line, old_id) {
                edits.push(RenameEdit {
                    file: span.file.to_string(),
                    line: line_no,
                    start_col: start,
                    end_col: end,
                    new_text: new_id.to_string(),
                });
            }
        }
    }
    Some(edits)
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

/// Whole-word occurrences of `needle` in `line` as (byte start, byte end).
fn word_occurrences(line: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let bytes = line.as_bytes();
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80;
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(pos) = line[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word(bytes[end]);
        if before_ok && after_ok {
            out.push((start, end));
        }
        from = end;
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
