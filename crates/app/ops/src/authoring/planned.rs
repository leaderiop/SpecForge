//! What a plan of a project's sources will write (ADR 0184): a typed edit's plan and a rename's plan each hold one
//! [`PlannedChange`], which answers the affected files, the after texts, the text edits and the diff, and writes
//! them as one project change (ADR 0128).

use crate::authoring::init::gitattributes_with;
use crate::change::ProjectChange;
use crate::read::navigate::{SourceText, TextEdit};
use crate::{Caller, OpError, OpErrorKind, Writes};
use specforge_common::{SourceRoot, Sym};
use specforge_project::CompiledProject;
use specforge_project::harness::{LEDGER_FILE, Ledger, LedgerChange};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Per file of the project: the text it was planned on (`None`: the file is new) and the text it will hold. A
/// file whose after text is its planned-on text is not in it; a new file reads as empty, so one left empty is not in
/// it either.
///
/// Each planned-on text is the text the project was compiled from; `new` is `pub(crate)`, so only ops' planners make
/// one. Files, edits and the diff are in key order.
#[derive(Debug, Clone)]
pub struct PlannedChange {
    sources: SourceRoot,
    /// What the operation was doing, the head of a failure's message (`cannot rename 'a'`, ADR 0155).
    doing: String,
    files: BTreeMap<String, (Option<Arc<str>>, String)>,
    /// The harness ledger the change rewrites (ADR 0287): a harness value a write moves or re-resolves stays recorded.
    ledger: Option<LedgerWrite>,
    /// The project declares a harness field: every write of it takes the write lock, the ledger changing or not.
    locked: bool,
}

type Redo = Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/// The rewrite of `specforge-harness` a change carries: what it was planned on, what it will hold, and how to make it
/// again over what the file holds when the change is applied, under the lock.
#[derive(Clone)]
struct LedgerWrite {
    planned_on: Vec<u8>,
    after: Vec<u8>,
    /// `.gitattributes` lacks the ledger's merge attribute: the write adds it.
    attributes: bool,
    redo: Redo,
}

impl std::fmt::Debug for LedgerWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedgerWrite").finish_non_exhaustive()
    }
}

impl PlannedChange {
    /// `files`: (path relative to the project root, planned-on text, after text); an unchanged one is dropped, and a new
    /// one whose after text is empty is unchanged.
    pub(crate) fn new(
        sources: SourceRoot,
        doing: impl Into<String>,
        files: impl IntoIterator<Item = (String, Option<Arc<str>>, String)>,
    ) -> Self {
        let files = files
            .into_iter()
            .filter(|(_, before, after)| before.as_deref().unwrap_or("") != after.as_str())
            .map(|(key, before, after)| (key, (before, after)))
            .collect();
        PlannedChange {
            sources,
            doing: doing.into(),
            files,
            ledger: None,
            locked: false,
        }
    }

    /// This change keeping the harness ledger true to the project `before` (ADR 0287): the ledger as the host would
    /// rewrite it for the edited project, `change` having moved or set values. Nothing when no extension declares a
    /// harness field or the ledger stays as it is.
    pub(crate) fn keeping_ledger(self, before: &CompiledProject, change: LedgerChange) -> Self {
        self.with_ledger(before, change, false)
    }

    /// This change writing the ledger canonically (`specforge harness gc`). Nothing when no loaded extension declares
    /// a harness field: with none, every field line reads as one of a field that is gone, and would be dropped.
    pub(crate) fn rewriting_ledger(self, before: &CompiledProject) -> Self {
        self.with_ledger(before, LedgerChange::default(), true)
    }

    fn with_ledger(mut self, before: &CompiledProject, change: LedgerChange, force: bool) -> Self {
        if before.root().is_none() || !before.declares_harness() {
            return self;
        }
        // Every write of this project takes the lock, the ledger changing or not.
        self.locked = before.serializes_writes();
        let current = before.root_file(LEDGER_FILE);
        let after = before.edited(&self.after());
        let bytes = before.rewritten_ledger(&after, &current, &change, false);
        let unchanged = bytes == current
            || (current.is_empty()
                && Ledger::parse(&String::from_utf8_lossy(&bytes))
                    .ledger
                    .is_empty());
        // A ledger that is written, or kept by `gc`, goes with its merge attribute.
        let attributes = (!unchanged || (force && !current.is_empty()))
            && gitattributes_with(&before.root_file(".gitattributes")).is_some();
        // A write that moves or sets a value keeps its ledger part though the ledger reads the same now: under the
        // lock the ledger may have changed, and the part is made again over what it holds.
        let keeps = !change.moves.is_empty() || !change.set.is_empty();
        if unchanged && !attributes && !keeps {
            return self;
        }
        let before = before.clone();
        self.ledger = Some(LedgerWrite {
            planned_on: current,
            after: bytes,
            attributes,
            redo: Arc::new(move |current| before.rewritten_ledger(&after, current, &change, true)),
        });
        self
    }

    /// The bytes the change will write to `specforge-harness`, when it rewrites it: what the edited project is
    /// judged over.
    pub(crate) fn ledger_after(&self) -> Option<&[u8]> {
        self.ledger.as_ref().map(|l| l.after.as_slice())
    }

    /// Nothing to write.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
            && !self
                .ledger
                .as_ref()
                .is_some_and(|l| l.after != l.planned_on || l.attributes)
    }

    /// The files the change writes, relative to the project root, in key order.
    pub fn affected_files(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str).chain(
            self.ledger
                .iter()
                .filter(|l| l.after != l.planned_on)
                .map(|_| LEDGER_FILE),
        )
    }

    /// Each file the change writes and the text it will hold, by path relative to the project root.
    pub fn after(&self) -> BTreeMap<String, String> {
        self.files
            .iter()
            .map(|(key, (_, after))| (key.clone(), after.clone()))
            .collect()
    }

    /// The edits that turn each file's planned-on text into the text it will hold, as navigation's `TextEdit`s:
    /// spans in the planned-on text (a new file's: one insertion at its start), each file's in order, none
    /// overlapping (the formatter's minimal diff, ADR 0021 D8), files in key order.
    pub fn text_edits(&self) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        for (key, (before, after)) in &self.files {
            let before = before.clone().unwrap_or_else(|| Arc::from(""));
            let text = SourceText::new(Arc::clone(&before));
            let file = Sym::new(key);
            edits.extend(
                crate::authoring::format::between(&before, after)
                    .into_iter()
                    .map(|change| TextEdit {
                        span: text.span(file, change.range.start, change.range.end),
                        new_text: change.new_text,
                    }),
            );
        }
        edits
    }

    /// The entities whose block the change creates, changes or deletes, sorted: each ID whose blocks (the file and
    /// the whole lines each spans, as the parser reads the planned-on and the after text of every file in the change)
    /// differ between the two. A block that only moves within its file is not named; one moved to another file is.
    pub fn entities(&self) -> Vec<String> {
        let (mut before, mut after) = (Blocks::new(), Blocks::new());
        for (key, (planned_on, text)) in &self.files {
            blocks(&mut before, key, planned_on.as_deref().unwrap_or(""));
            blocks(&mut after, key, text);
        }
        let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        ids.into_iter()
            .filter(|id| before.get(*id) != after.get(*id))
            .cloned()
            .collect()
    }

    /// A unified diff of every file, project-root-relative headers, in key order. Empty when nothing changes.
    pub fn diff(&self) -> String {
        let ledger = self.ledger.iter().map(|l| {
            specforge_formatter::unified_diff(
                LEDGER_FILE,
                &String::from_utf8_lossy(&l.planned_on),
                &String::from_utf8_lossy(&l.after),
            )
            .diff_text
        });
        let mut diffs: Vec<(&str, String)> = self
            .files
            .iter()
            .map(|(key, (before, after))| {
                (
                    key.as_str(),
                    specforge_formatter::unified_diff(key, before.as_deref().unwrap_or(""), after)
                        .diff_text,
                )
            })
            .collect();
        diffs.extend(ledger.map(|diff| (LEDGER_FILE, diff)));
        diffs.sort_by_key(|(key, _)| *key);
        diffs.into_iter().map(|(_, diff)| diff).collect()
    }

    /// Write the change as one project change (ADR 0128), each file resolved through the caller's containment (ADR
    /// 0104): checked against the text it was planned on, then replaced or created; on any failure, every file
    /// already written is put back (one that cannot be is the error's writes). Returns the files it wrote.
    pub(crate) fn apply(&self, caller: Caller<'_>) -> Result<Writes, OpError> {
        // A project with no harness field has no ledger to serialize: it gets no `.specforge/` from a write.
        let _lock = (self.locked || self.ledger.is_some())
            .then(|| specforge_project::harness::lock(self.sources.base()))
            .transpose()
            .map_err(|e| {
                OpError::new(
                    OpErrorKind::Internal,
                    crate::FILE_UNWRITABLE,
                    format!("{}: cannot take the write lock: {e}", self.doing),
                )
            })?;
        let mut change = ProjectChange::new(self.sources.base(), self.doing.as_str());
        for (key, (before, after)) in &self.files {
            let held = caller.containment().resolve(&self.sources.path(key))?;
            change.stage(
                held,
                before.as_ref().map(|b| b.as_bytes().to_vec()),
                after.clone().into_bytes(),
            );
        }
        if let Some(ledger) = &self.ledger {
            let held = caller
                .containment()
                .resolve(&self.sources.path(LEDGER_FILE))?;
            let current = held.read_if_there().ok().flatten();
            let now = current.clone().unwrap_or_default();
            let after = if now == ledger.planned_on {
                ledger.after.clone()
            } else {
                (ledger.redo)(&now)
            };
            if after != now {
                change.stage(held, current, after);
            }
            if ledger.attributes {
                let held = caller
                    .containment()
                    .resolve(&self.sources.path(".gitattributes"))?;
                let current = held.read_if_there().ok().flatten();
                if let Some(after) = gitattributes_with(current.as_deref().unwrap_or_default()) {
                    change.stage(held, current, after);
                }
            }
        }
        change.commit()
    }
}

/// Each entity's blocks, by ID: (file, the whole lines the block spans), in file then text order.
type Blocks = BTreeMap<String, Vec<(String, String)>>;

/// The blocks of `text` (the file `key`) into `into`. A span a recovered parse ends past the text is cut at its end.
fn blocks(into: &mut Blocks, key: &str, text: &str) {
    let lines: Vec<&str> = text.split('\n').collect();
    for entity in specforge_parser::parse(text, key).entities {
        let end = entity.span.end_line.min(lines.len());
        let start = entity.span.start_line.saturating_sub(1).min(end);
        into.entry(entity.id.raw.to_string())
            .or_default()
            .push((key.to_string(), lines[start..end].join("\n")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn text(s: &str) -> Option<Arc<str>> {
        Some(Arc::from(s))
    }

    fn change(files: Vec<(&str, Option<Arc<str>>, &str)>) -> PlannedChange {
        PlannedChange::new(
            SourceRoot::new(
                PathBuf::from("/root"),
                PathBuf::from("/root"),
                specforge_common::Exclude::default(),
            ),
            "cannot test",
            files
                .into_iter()
                .map(|(k, b, a)| (k.to_string(), b, a.to_string())),
        )
    }

    #[test]
    fn a_file_whose_after_is_its_planned_on_text_is_not_in_the_change() {
        let same = change(vec![("a.spec", text("x\n"), "x\n")]);
        assert!(same.is_empty());
        assert_eq!(same.affected_files().count(), 0);
        assert!(same.after().is_empty() && same.diff().is_empty() && same.text_edits().is_empty());
        let one = change(vec![
            ("a.spec", text("x\n"), "x\n"),
            ("b.spec", text("x\n"), "y\n"),
        ]);
        assert_eq!(one.affected_files().collect::<Vec<_>>(), ["b.spec"]);
    }

    #[test]
    fn affected_files_and_the_diff_are_in_key_order() {
        let c = change(vec![
            ("b.spec", None, "b\n"),
            ("a/x.spec", None, "x\n"),
            ("a.spec", None, "a\n"),
        ]);
        assert_eq!(
            c.affected_files().collect::<Vec<_>>(),
            ["a.spec", "a/x.spec", "b.spec"]
        );
        let diff = c.diff();
        let at = |name: &str| diff.find(name).unwrap();
        assert!(at("a.spec") < at("a/x.spec") && at("a/x.spec") < at("b.spec"));
    }

    #[test]
    fn a_new_file_left_empty_is_not_in_the_change() {
        let c = change(vec![("a.spec", None, ""), ("b.spec", None, "b\n")]);
        assert_eq!(c.affected_files().collect::<Vec<_>>(), ["b.spec"]);
        assert!(change(vec![("a.spec", None, "")]).is_empty());
    }

    #[test]
    fn entities_are_the_blocks_that_differ() {
        let c = change(vec![
            (
                "a.spec",
                text("type a \"A\" {\n}\n\ntype b \"B\" {\n}\n"),
                "type a \"A\" {\n}\n\ntype b \"Bee\" {\n}\n",
            ),
            ("c.spec", None, "type c \"C\" {\n}\n"),
        ]);
        assert_eq!(c.entities(), ["b", "c"]);
    }

    #[test]
    fn a_block_moved_to_another_file_is_named_and_one_moved_within_its_file_is_not() {
        let c = change(vec![
            (
                "a.spec",
                text("type x \"X\" {\n}\n\ntype y \"Y\" {\n}\n"),
                "// above\ntype y \"Y\" {\n}\n",
            ),
            ("b.spec", None, "type x \"X\" {\n}\n"),
        ]);
        assert_eq!(c.entities(), ["x"]);
    }
}
