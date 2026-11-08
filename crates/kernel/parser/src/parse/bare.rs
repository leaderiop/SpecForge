//! A reference holding `:` written without its quotes (ADR 0346 D6): `notes [js:p1]`. The grammar reads `js`, then
//! fails on `:p1`, and its recovery takes many shapes (`js` and an error, an error and `p1`, a field `p1` of its own
//! for a single value). Each is one E120 spanning the whole `word:rest`, which says to quote it; no E001 for what it
//! covers, and no word of it stays in the value, so nothing is linked as `js`.

use super::*;

/// One leaf of a block body as the bare-reference rule reads it.
struct Leaf<'t> {
    node: Node<'t>,
    /// Bytes a reference may hold (no space, quote, bracket, brace, parenthesis or comma): a run is made of these.
    part: bool,
    /// Under an ERROR node, or one itself: the grammar did not read it.
    error: bool,
    /// Under a field's value.
    value: bool,
}

impl<'a> ParseContext<'a> {
    /// E120 for each reference holding `:` that a value of the block body `body` writes bare: a run of adjacent
    /// leaves, the grammar failing on part of it, that starts in a field's value and spells `word:rest`. A nested
    /// block's body is its own. Recorded in `bare`, which the body's fields read to drop what the run covers.
    pub(super) fn report_bare_references(&mut self, body: Node) {
        let mut leaves = Vec::new();
        self.leaves(body, false, false, &mut leaves);
        let mut start = 0;
        while start < leaves.len() {
            let mut end = start + 1;
            if leaves[start].part {
                while end < leaves.len()
                    && leaves[end].part
                    && leaves[end - 1].node.end_byte() == leaves[end].node.start_byte()
                {
                    end += 1;
                }
                self.bare_run(&leaves[start..end]);
            }
            start = end;
        }
    }

    /// Whether `span` lies within a reference written bare.
    pub(super) fn in_bare(&self, span: &SourceSpan) -> bool {
        self.bare.iter().any(|bare| errors::span_within(span, bare))
    }

    /// E120 for `run` when it is one bare reference.
    fn bare_run(&mut self, run: &[Leaf<'_>]) {
        let (first, last) = (&run[0], &run[run.len() - 1]);
        if !first.value || !run.iter().any(|leaf| leaf.error) {
            return;
        }
        let text = &self.source[first.node.start_byte()..last.node.end_byte()];
        let Some((word, rest)) = text.split_once(':') else {
            return;
        };
        let worded = word
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if !worded || rest.is_empty() {
            return;
        }
        let (from, to) = (self.span(first.node), self.span(last.node));
        let span = SourceSpan {
            end_line: to.end_line,
            end_col: to.end_col,
            ..from
        };
        self.errors.push(ParseError {
            code: specforge_common::codes::E120,
            message: format!("a reference holding ':' is written quoted: \"{text}\""),
            span: span.clone(),
            suggestion: Some(format!("write \"{text}\"")),
        });
        self.bare.push(span);
    }

    /// The leaves under `node`, in source order, each read as [`Leaf`] says. A string, a comment and a nested block
    /// are one leaf that holds no reference.
    fn leaves<'t>(&self, node: Node<'t>, error: bool, value: bool, out: &mut Vec<Leaf<'t>>) {
        let error = error || node.is_error();
        let opaque = matches!(
            node.kind(),
            kind::STRING
                | kind::TRIPLE_QUOTED_STRING
                | kind::COMMENT
                | kind::NESTED_BLOCK
                | kind::BLOCK_LIST
        );
        if node.is_missing() {
            return;
        }
        if opaque || node.child_count() == 0 {
            let text = self.text(node);
            let part = !opaque
                && !text.is_empty()
                && !text
                    .chars()
                    .any(|c| c.is_whitespace() || "\"{}()[],".contains(c));
            out.push(Leaf {
                node,
                part,
                error,
                value,
            });
            return;
        }
        let held = (node.kind() == kind::FIELD)
            .then(|| node.child_by_field_name(field::VALUE))
            .flatten();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let value = value || held.is_some_and(|held| held.id() == child.id());
            self.leaves(child, error, value, out);
        }
    }
}
