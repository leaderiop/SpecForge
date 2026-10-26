//! A block's doc comment: the run of whole-line comments directly above its first line, the format header never one
//! (ADR 0202); and a member's, by the same rule (ADR 0248); and the comments inside and after a member's value
//! (ADR 0267).

use super::*;

/// The run of `comments` (whole-line, in source order) directly above 1-based `line`: the last on the line above it,
/// each on the line below the one before, no blank line between, none on a 0-based row `stop` refuses. From the first
/// one's start to the last one's end; `None` when no comment stands on the line above.
fn run_above(
    comments: &[SourceSpan],
    line: usize,
    stop: impl Fn(usize) -> bool,
) -> Option<SourceSpan> {
    let below = comments.partition_point(|c| c.start_line < line);
    let run = comments[..below]
        .iter()
        .rev()
        .zip((1..line).rev())
        .take_while(|(c, above)| c.start_line == *above && !stop(above - 1))
        .count();
    (run > 0).then(|| {
        let (first, last) = (&comments[below - run], &comments[below - 1]);
        SourceSpan {
            file: first.file,
            start_line: first.start_line,
            start_col: first.start_col,
            end_line: last.end_line,
            end_col: last.end_col,
        }
    })
}

/// Each of `entities` given its doc comment, read from `comments` (whole-line comments between top-level items, in
/// source order, absolute spans) and `header` (the format header's 0-based row, ADR 0095).
pub(super) fn attach(entities: &mut [Entity], comments: &[SourceSpan], header: Option<usize>) {
    for entity in entities {
        entity.doc_span = run_above(comments, entity.span.start_line, |row| header == Some(row));
    }
}

impl Entity {
    /// Where the comment lines of the member that starts on 1-based `line` are written: the run of
    /// [`Self::body_comments`] directly above it, by the rule of [`Self::doc_span`] (ADR 0202, ADR 0248). `None`
    /// when none stands there. Lines are as this parse read them.
    pub fn comments_above(&self, line: usize) -> Option<SourceSpan> {
        run_above(&self.body_comments, line, |_| false)
    }

    /// The comment trailing the member whose value is written at `value`: on the row the value ends on, past it (after
    /// its annotations). `None` when none stands there.
    pub fn trailing_comment(&self, value: &SourceSpan) -> Option<&SourceSpan> {
        self.trailing_comments
            .iter()
            .find(|c| c.start_line == value.end_line && c.start_col >= value.end_col)
    }

    /// Whether a comment is written inside `value` (a member's value span): a list's, a nested block field's member
    /// comment or trailing comment. The comment trailing the value itself is outside it ([`Self::trailing_comment`]).
    pub fn holds_comment(&self, value: &SourceSpan) -> bool {
        let inside = |c: &SourceSpan| {
            (c.start_line, c.start_col) > (value.start_line, value.start_col)
                && (c.start_line, c.start_col) < (value.end_line, value.end_col)
        };
        self.value_comments
            .iter()
            .chain(&self.body_comments)
            .chain(&self.trailing_comments)
            .any(inside)
    }
}
