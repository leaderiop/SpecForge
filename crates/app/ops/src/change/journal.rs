//! The write journal (ADR 0289): under the write lock a write records what it is about to replace, and removes the
//! record when every file is written. A record still there at the next lock is an interrupted write: [`rollback`]
//! puts back each file that still holds what the write left, leaves one that holds what it replaced, and leaves and
//! names one that holds neither. A rollback never rolls forward: the interrupted plan is stale by then, and its
//! caller plans again. The format is `specforge_project::journal`'s, which a check reads as W167.

use crate::{Caller, OpError, OpErrorKind, Writes};
use specforge_common::SourceRoot;
use specforge_project::journal::{Entry, FORMAT, JOURNAL_FILE, Journal, digest};
use std::path::PathBuf;

/// What a record says a removed file holds after the write: nothing.
const ABSENT: &str = "absent";

/// What a write is about to replace, collected as its files are staged.
pub(crate) struct Record {
    root: PathBuf,
    entries: Vec<Entry>,
    /// A file that is not UTF-8 text cannot be recorded: the write then goes unjournaled.
    // shortcut: a non-UTF-8 file is journaled as nothing; upgrade if a source may be binary.
    complete: bool,
}

impl Record {
    pub(crate) fn new(sources: &SourceRoot) -> Self {
        Record {
            root: sources.base().join(JOURNAL_FILE),
            entries: Vec::new(),
            complete: true,
        }
    }

    /// The file `path` (from the project root) held `before` (`None`: it was not there) and will hold `after`.
    pub(crate) fn add(&mut self, path: &str, before: Option<&[u8]>, after: &[u8]) {
        let before = match before.map(std::str::from_utf8) {
            None => None,
            Some(Ok(text)) => Some(text.to_string()),
            Some(Err(_)) => {
                self.complete = false;
                return;
            }
        };
        self.entries.push(Entry {
            path: path.to_string(),
            before,
            after: digest(after),
        });
    }

    /// The file `path` held `before` and will be removed (ADR 0292: install removes what it marked).
    pub(crate) fn add_removal(&mut self, path: &str, before: &[u8]) {
        let Ok(text) = std::str::from_utf8(before) else {
            self.complete = false;
            return;
        };
        self.entries.push(Entry {
            path: path.to_string(),
            before: Some(text.to_string()),
            after: ABSENT.to_string(),
        });
    }

    /// Write the record, synced, before the first file is replaced.
    pub(crate) fn begin(&self, doing: &str) -> Result<(), OpError> {
        if !self.complete || self.entries.is_empty() {
            return Ok(());
        }
        let journal = Journal {
            format: FORMAT,
            doing: doing.to_string(),
            files: self.entries.clone(),
            done: false,
        };
        let result = self
            .root
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| specforge_common::fresh::replace_file(&self.root, &journal.to_bytes()));
        result.map_err(|e| {
            OpError::new(
                OpErrorKind::of_io(&e),
                crate::FILE_UNWRITABLE,
                format!("{doing}: cannot write the write journal: {e}"),
            )
        })
    }

    /// Every file was put back, or none was written: the record is not needed.
    pub(crate) fn discard(&self) {
        let _ = std::fs::remove_file(&self.root);
    }

    /// Every file is written: the record says so, synced, then goes. A record left behind by a crash or by a failed
    /// removal says `done`, and the next writer only removes it, so a completed write is never rolled back.
    pub(crate) fn finish(&self, doing: &str) -> Result<(), OpError> {
        if !self.complete || self.entries.is_empty() {
            return Ok(());
        }
        let done = Journal {
            format: FORMAT,
            doing: doing.to_string(),
            files: self.entries.clone(),
            done: true,
        };
        let marked = specforge_common::fresh::replace_file(&self.root, &done.to_bytes());
        match std::fs::remove_file(&self.root) {
            Ok(()) => Ok(()),
            Err(e) => Err(OpError::new(
                OpErrorKind::of_io(&e),
                crate::FILE_UNWRITABLE,
                format!(
                    "{doing}: the write completed but its journal cannot be removed ({e}){}",
                    match marked {
                        Ok(()) =>
                            "; it is marked done, so the next write only clears it".to_string(),
                        Err(m) => format!(
                            " and cannot be marked done ({m}): the next write would roll it back"
                        ),
                    }
                ),
            )),
        }
    }
}

/// Whether a rollback may touch `entry`: a source, the ledger or `.gitattributes` inside the project root (no
/// absolute path, no `..`), whose `after` is a digest.
fn restorable(entry: &Entry) -> bool {
    use std::path::Component;
    let path = std::path::Path::new(&entry.path);
    let inside = path.components().all(|c| matches!(c, Component::Normal(_)))
        && path.components().next().is_some();
    let kind = entry.path.ends_with(".spec")
        || entry.path == specforge_project::harness::LEDGER_FILE
        || entry.path == ".gitattributes"
        || crate::host::owned_path(&entry.path);
    let digest = entry.after == ABSENT
        || entry.after.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        });
    inside && kind && digest
}

/// Roll back the interrupted write the journal at `sources`' project root records, if there is one: the files put
/// back are the returned writes. Called under the write lock.
pub(crate) fn rollback(sources: &SourceRoot, caller: Caller<'_>) -> Result<Writes, OpError> {
    let at = sources.base().join(JOURNAL_FILE);
    let journal_file = caller.containment().resolve(&at)?;
    let Some(bytes) = journal_file.read_if_there().ok().flatten() else {
        return Ok(Writes::none());
    };
    let mut put_back = Writes::none();
    // A journal that is not one (another format, a torn write) records nothing to put back.
    // A journal marked done records a finished write: it is only removed.
    if let Some(journal) = Journal::parse(&bytes).filter(|journal| !journal.done) {
        // A journal is a file in the project directory, so it is not trusted: it restores only what a write puts there.
        if let Some(entry) = journal.files.iter().find(|e| !restorable(e)) {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                crate::FILE_UNWRITABLE,
                format!(
                    "the write journal names '{}', which no write of this project records; \
                     remove {JOURNAL_FILE} if you did not expect it",
                    entry.path
                ),
            ));
        }
        for entry in &journal.files {
            let held = caller.containment().resolve(&sources.path(&entry.path))?;
            let now = held.read_if_there().map_err(|e| {
                OpError::new(
                    OpErrorKind::of_io(&e),
                    crate::FILE_UNREADABLE,
                    format!(
                        "cannot read {} to roll back '{}': {e}",
                        entry.path, journal.doing
                    ),
                )
            })?;
            let at_after = if entry.after == ABSENT {
                now.is_none()
            } else {
                now.as_deref().map(digest).as_deref() == Some(entry.after.as_str())
            };
            if !at_after {
                // At `before`, or at neither: the file is left alone.
                continue;
            }
            let restored = match &entry.before {
                Some(text) if now.is_some() => held.replace(text),
                Some(text) => held.create_parent().and_then(|()| held.create_new(text)),
                None => held.remove(),
            };
            restored.map_err(|e| {
                OpError::new(
                    OpErrorKind::of_io(&e),
                    crate::FILE_UNWRITABLE,
                    format!(
                        "cannot roll back {} of '{}': {e}",
                        entry.path, journal.doing
                    ),
                )
            })?;
            put_back.record(sources.path(&entry.path));
        }
    }
    journal_file.remove().map_err(|e| {
        OpError::new(
            OpErrorKind::of_io(&e),
            crate::FILE_UNWRITABLE,
            format!("cannot remove the write journal: {e}"),
        )
    })?;
    Ok(put_back)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use specforge_common::Exclude;
    use specforge_test_macros::test as specforge_test;

    /// A file of the random change: what it held (`None`: absent), what the write gives it, and whether the
    /// interruption has reached it; `other`, if any, is a text another writer gave it afterwards.
    #[derive(Debug, Clone)]
    struct Planned {
        before: Option<String>,
        after: String,
        other: Option<String>,
    }

    fn planned() -> impl Strategy<Value = Planned> {
        (
            proptest::option::of("[a-z]{0,6}"),
            "[A-Z]{1,6}",
            proptest::option::of("[0-9]{1,6}"),
        )
            .prop_map(|(before, after, other)| Planned {
                before,
                after,
                other,
            })
    }

    fn put(path: &std::path::Path, text: &Option<String>) {
        match text {
            Some(text) => std::fs::write(path, text).unwrap(),
            None => {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        #[specforge_test(
            behavior = "roll_back_interrupted_write",
            verify = "rollback restores exactly the files at their after digest"
        )]
        fn rollback_restores_exactly_the_files_at_their_after_digest(
            files in proptest::collection::vec(planned(), 1..6),
            written in 0usize..7,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            let sources = SourceRoot::new(root.to_path_buf(), root.to_path_buf(), Exclude::default());
            let mut record = Record::new(&sources);
            for (n, file) in files.iter().enumerate() {
                let path = format!("f{n}.spec");
                put(&root.join(&path), &file.before);
                record.add(&path, file.before.as_deref().map(str::as_bytes), file.after.as_bytes());
            }
            record.begin("cannot test").unwrap();
            // The write is killed after `written` of its files; another writer then changes some file.
            let mut expected = Vec::new();
            for (n, file) in files.iter().enumerate() {
                let path = root.join(format!("f{n}.spec"));
                if n < written {
                    put(&path, &Some(file.after.clone()));
                }
                let mut now = if n < written { Some(file.after.clone()) } else { file.before.clone() };
                let mut want = file.before.clone();
                if let Some(other) = &file.other {
                    put(&path, &Some(other.clone()));
                    now = Some(other.clone());
                    want = now.clone();
                } else if n >= written {
                    want = now.clone();
                }
                let _ = now;
                expected.push(want);
            }
            rollback(&sources, crate::testing::caller()).unwrap();
            for (n, want) in expected.iter().enumerate() {
                let got = std::fs::read_to_string(root.join(format!("f{n}.spec"))).ok();
                prop_assert_eq!(&got, want, "file {}", n);
            }
            prop_assert!(!root.join(JOURNAL_FILE).exists());
        }
    }
}

#[cfg(test)]
mod owned_tests;
