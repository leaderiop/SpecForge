//! The `Fs` port: stat, read and walk a path under a named root (ADR 0335 D1). Its adapter is `specforge-fs`, the
//! real file system behind [`crate::contained`].

use std::sync::Arc;

use specforge_protocol_types::{FsRequest, OutcomeClass};

use crate::contained::landed_in_host_state;
use crate::{Contained, Lookup, Resolved, Root, Roots};

/// The most bytes one read answers.
pub const READ_CEILING: u64 = 1024 * 1024;
/// A file larger than this is refused unread (ADR 0264).
pub const FILE_CEILING: u64 = 64 * 1024 * 1024;
/// The most entries a walk visits (ADR 0293 D4).
pub const WALK_VISITS: usize = 20_000;
/// Directories a walk or a glob does not enter unless a literal segment of the pattern names them (ADR 0293 D4):
/// version control, the tool's own state, and the usual build and dependency output. An `Fs` walk never enters the
/// tool's own state, named or not ([`crate::contained::in_host_state`]).
pub const SKIPPED: &[&str] = &[
    ".git",
    ".specforge",
    ".next",
    ".venv",
    "dist",
    "node_modules",
    "target",
    "vendor",
    ".svn",
];

/// Performs one file effect. Every adapter resolves the request's path in its root first: a path that leaves it is
/// `outside`, a withheld name `denied`, an absent one `missing`.
pub trait Fs: Send + Sync {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome;

    /// A person's listing (`specforge files list`, ADR 0339): no guest names one, so only the local adapter answers it
    /// and every other port, a granted one included, answers `denied`.
    fn list(&self, _roots: &Roots, _request: &ListRequest) -> FsListing {
        FsListing::of(OutcomeClass::Denied)
    }

    /// A person's change (`specforge files mkdir`, `copy`, `move`, `remove`, `temp`): host-only like [`Fs::list`].
    fn change(&self, _roots: &Roots, _change: &FsChange) -> FsChanged {
        FsChanged::of(OutcomeClass::Denied, "this port changes no file")
    }

    /// The SHA-256 of a whole file, streamed (`specforge files digest`): host-only like [`Fs::list`].
    fn digest(&self, _roots: &Roots, _at: &Place) -> FsDigest {
        FsDigest::of(OutcomeClass::Denied)
    }
}

/// A path in a root, as a person's change names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub root: String,
    /// Relative to the root, `/` separated.
    pub path: String,
}

/// A change a person asks of the file system. Host-only: no wire request carries one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsChange {
    /// The directory and its missing parents.
    Mkdir { at: Place },
    /// A new empty file, or directory, refused when the name exists.
    Create { at: Place, dir: bool },
    /// A file, or a directory and what it holds (no link), onto `to`, or into it when it is a directory.
    Copy { from: Place, to: Place },
    /// As [`FsChange::Copy`], then `from` is gone.
    Move { from: Place, to: Place },
    /// A file, or an empty directory, or with `recursive` a directory and what it holds; never a root itself.
    Remove { at: Place, recursive: bool },
}

/// What a change did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsChanged {
    pub class: OutcomeClass,
    /// The paths it made, wrote or removed, root-relative.
    pub changed: Vec<Place>,
    /// Why it was refused, for a person.
    pub detail: Option<String>,
}

impl FsChanged {
    /// A refusal of `class`, `detail` saying why.
    pub fn of(class: OutcomeClass, detail: impl Into<String>) -> Self {
        FsChanged {
            class,
            changed: Vec::new(),
            detail: Some(detail.into()),
        }
    }
}

/// A file's digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsDigest {
    pub class: OutcomeClass,
    /// Lowercase hex.
    pub sha256: String,
    pub bytes: u64,
}

impl FsDigest {
    pub fn of(class: OutcomeClass) -> Self {
        FsDigest {
            class,
            sha256: String::new(),
            bytes: 0,
        }
    }
}

/// The most entries a person's listing visits.
pub const LIST_VISITS: usize = 200_000;

/// What a person's listing asks: the entries under `path` in `root`. Host-only: no wire request carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRequest {
    pub root: String,
    /// Relative to the root, `/` separated; empty for the root itself.
    pub path: String,
    /// A listed entry matches one of them, relative to `path` (a glob with no `/` matches the entry's name); none:
    /// every entry.
    pub globs: Vec<String>,
    /// Only entries of this kind; none: files and directories.
    pub kind: Option<FileKind>,
    /// How deep below `path` an entry may be (1: its direct entries); none: any depth.
    pub depth: Option<usize>,
    /// Leave out what `.gitignore`, `.ignore` and git's own excludes ignore.
    pub ignore: bool,
    /// List names that start with `.`.
    pub hidden: bool,
    pub max_entries: usize,
}

/// One entry a listing found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Relative to the root, `/` separated.
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
}

/// What a listing answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsListing {
    pub class: OutcomeClass,
    /// Sorted by path.
    pub entries: Vec<Listed>,
    /// It stopped at `max_entries` or at [`LIST_VISITS`].
    pub truncated: bool,
}

impl FsListing {
    /// A listing of `class` and no entry.
    pub fn of(class: OutcomeClass) -> Self {
        FsListing {
            class,
            entries: Vec::new(),
            truncated: false,
        }
    }
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

/// Where `request` leads in `roots`, as every `Fs` adapter decides it ([`Contained`] over `lookup`): the whole
/// [`Resolved`] (its root, root-relative path, where it lands and whether the written name or the landing is
/// withheld), the root itself an empty path; or the class of a refusal (`outside`, `denied` for anything in the
/// host's own state, whatever the op) or of a path that is absent as written (`missing`, a case variant included).
pub fn held(
    roots: &Roots,
    lookup: &dyn Lookup,
    request: &FsRequest,
) -> Result<Resolved, OutcomeClass> {
    let name = Root::parse(&request.root).ok_or(OutcomeClass::Outside)?;
    let root = roots.get(&name).ok_or(OutcomeClass::Outside)?.to_path_buf();
    if matches!(request.path.as_str(), "" | "." | "./") {
        // A named root can itself be a directory of the project's own state.
        if !roots.is_temp(&name)
            && lookup
                .canonical(&root)
                .is_some_and(|real| landed_in_host_state(roots, lookup, &real))
        {
            return Err(OutcomeClass::Denied);
        }
        return Ok(Resolved {
            root: name,
            rel: String::new(),
            at: root,
            real: None,
            exists: true,
            case: None,
            backslashes: false,
            withheld: false,
            host_state: false,
        });
    }
    match Contained::new(roots, lookup).resolve(&request.path, name) {
        Ok(resolved) if resolved.host_state => Err(OutcomeClass::Denied),
        Ok(resolved) if resolved.exists && resolved.case.is_none() => Ok(resolved),
        Ok(_) => Err(OutcomeClass::Missing),
        Err(_) => Err(OutcomeClass::Outside),
    }
}
