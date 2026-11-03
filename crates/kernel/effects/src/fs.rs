//! The `Fs` port: stat, read and walk a path under a named root (ADR 0335 D1). Its adapter is `specforge-fs`, the
//! real file system behind [`crate::contained`].

use std::sync::Arc;

use specforge_protocol_types::{FsRequest, OutcomeClass};

use crate::Roots;

/// The most bytes one read answers.
pub const READ_CEILING: u64 = 1024 * 1024;
/// A file larger than this is refused unread (ADR 0264).
pub const FILE_CEILING: u64 = 64 * 1024 * 1024;
/// The most entries a walk visits (ADR 0293 D4).
pub const WALK_VISITS: usize = 20_000;
/// Directories a walk does not enter unless the glob names them (ADR 0293 D4).
pub const SKIPPED: &[&str] = &[".git", "node_modules", "target", ".specforge"];

/// Performs one file effect. Every adapter resolves the request's path in its root first: a path that leaves it is
/// `outside`, a withheld name `denied`, an absent one `missing`.
pub trait Fs: Send + Sync {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome;
}

/// A file port as the composition root shares it.
pub type SharedFs = Arc<dyn Fs>;

/// What a file effect answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsOutcome {
    pub class: OutcomeClass,
    /// For a stat (and a read) of a path that exists.
    pub stat: Option<FsStat>,
    /// A read's bytes, at most its `max_bytes` (held to [`READ_CEILING`]).
    pub bytes: Vec<u8>,
    /// The read stopped at `max_bytes`, or the walk at its budget.
    pub truncated: bool,
    /// A walk's matches: root-relative, `/` separated, sorted.
    pub entries: Vec<String>,
}

impl FsOutcome {
    /// An answer of `class` and nothing else.
    pub fn of(class: OutcomeClass) -> Self {
        FsOutcome {
            class,
            stat: None,
            bytes: Vec::new(),
            truncated: false,
            entries: Vec::new(),
        }
    }
}

/// What a path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    File,
    Dir,
    Other,
}

/// A path's kind, size and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsStat {
    pub kind: FileKind,
    pub size: u64,
    /// Milliseconds since the Unix epoch, when the file system keeps it.
    pub modified_ms: Option<u64>,
}
