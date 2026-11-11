//! The names a rename wrote in the spec files it edits besides the declaring one (ADR 0346 D7): a reference to the
//! renamed entity, or a path through it that places what it scopes (`milestone v1.1` in a phase's file, `phase v1.1/04`
//! in a test's). An editor undoes per buffer, so each of these files says on its own whether it holds the rename.

use std::collections::BTreeMap;
use std::sync::Arc;

use specforge_ops::read::navigate::{TextEdit, apply_edits};
use specforge_project::CompiledProject;
use specforge_project::facts::{Span, Write, Writes};

use super::reads::{range, reads};

/// One spec file a rename rewrote: the text it read and the text it wrote, and each name's place in each.
pub(super) struct Spelled {
    read: Arc<str>,
    wrote: String,
    read_at: Vec<Span>,
    wrote_at: Vec<Span>,
}

/// What a file says of a rename's names at their places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Says {
    /// The name it wrote, at each place it wrote it.
    Written,
    /// The name it read, at each place it read it: the file went back.
    Read,
    /// Neither: the places were edited otherwise, or are gone.
    Other,
}

/// Each file `edits` rewrites that `spelled` admits, read as `before` holds it: a file whose edits do not lie in that
/// text is left out.
pub(super) fn of(
    before: &CompiledProject,
    edits: &[TextEdit],
    spelled: impl Fn(&str) -> bool,
) -> BTreeMap<String, Spelled> {
    let mut by_file: BTreeMap<&str, Vec<&TextEdit>> = BTreeMap::new();
    for edit in edits.iter().filter(|e| spelled(e.span.file.as_str())) {
        by_file
            .entry(edit.span.file.as_str())
            .or_default()
            .push(edit);
    }
    let mut files = BTreeMap::new();
    for (file, edits) in by_file {
        let Some(read) = before.source_text(file) else {
            continue;
        };
        let span = |e: &TextEdit| {
            let s = &e.span;
            (s.start_line, s.start_col, s.end_line, s.end_col)
        };
        let writes = edits.iter().map(|e| {
            let held = range(&read, &span(e)).and_then(|r| read.get(r));
            Some(Write {
                span: span(e),
                held: held?.to_string(),
                now: e.new_text.clone(),
            })
        });
        let Some(writes) = writes.collect::<Option<Vec<_>>>().and_then(Writes::of) else {
            continue;
        };
        let read_at: Vec<Span> = edits.iter().map(|e| span(e)).collect();
        let wrote_at = read_at.iter().map(|s| writes.written(s)).collect();
        let (Some(wrote), Some(wrote_at)) = (apply_edits(&read, edits.iter().copied()), wrote_at)
        else {
            continue;
        };
        files.insert(
            file.to_string(),
            Spelled {
                read,
                wrote,
                read_at,
                wrote_at,
            },
        );
    }
    files
}

impl Spelled {
    /// What `now`, this file's text, says at the places of a rename from `old` to `new`.
    pub(super) fn says(&self, now: &str, old: &str, new: &str) -> Says {
        if reads(&self.wrote, now, &self.wrote_at, new) {
            Says::Written
        } else if reads(&self.read, now, &self.read_at, old) {
            Says::Read
        } else {
            Says::Other
        }
    }
}
