//! The write journal (ADR 0289): a write records what it is about to replace in `.specforge/write-journal.json`,
//! under the write lock, and removes it when every file is written. A journal still there means a write was
//! interrupted: the next writer puts back what it left half done, and until then a check reports it (W167) and never
//! rolls it back itself.
//!
//! ```text
//! {"format":1,"doing":"cannot rename 'a'","files":[{"path":"a.spec","before":"<text>"|null,"after":"sha256:.."}]}
//! ```
//! `path` is from the project root, `before` the text the write replaced (`null`: the file was not there) and `after`
//! the digest of what it wrote. A write that has committed every file sets `"done":true` before it removes the
//! record, so a record that survives a crash or a failed removal is never mistaken for an unfinished write.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, codes};
use specforge_package::Sha256;

/// The journal, from the project root.
pub const JOURNAL_FILE: &str = ".specforge/write-journal.json";

/// The only format there is.
pub const FORMAT: u32 = 1;

/// A write's record of what it replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub format: u32,
    /// What the write was doing, the head of its failure's message (`cannot rename 'a'`).
    pub doing: String,
    pub files: Vec<Entry>,
    /// Every file was written: the record is only waiting to be removed, and nothing is rolled back.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

/// One file the write replaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub before: Option<String>,
    pub after: String,
}

impl Journal {
    /// The journal `bytes` hold; none when they are not one of format [`FORMAT`].
    pub fn parse(bytes: &[u8]) -> Option<Journal> {
        serde_json::from_slice::<Journal>(bytes)
            .ok()
            .filter(|journal| journal.format == FORMAT)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("a journal serializes")
    }
}

/// The digest a journal records for the bytes a write left: `sha256:` and 64 lowercase hex.
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", Sha256::of(bytes))
}

/// W167 when the project at `root` has a journal, read through the checks' `disk`, so the check records it as an
/// input.
pub(crate) fn pending(root: &Path, disk: &dyn specforge_registry::Disk) -> Option<Diagnostic> {
    let journal = Journal::parse(&disk.read(&root.join(JOURNAL_FILE)).ok()?)?;
    if journal.done {
        return None;
    }
    let files: Vec<&str> = journal.files.iter().map(|f| f.path.as_str()).collect();
    Some(
        Diagnostic::new(
            codes::W167,
            format!(
                "an interrupted write is pending rollback: '{}' did not finish ({})",
                journal.doing,
                files.join(", ")
            ),
        )
        .with_suggestion(
            "the next SpecForge write rolls it back; until then these files may hold part of it",
        ),
    )
}
