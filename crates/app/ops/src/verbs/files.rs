//! `specforge files` (ADR 0339): stat, list, search, change and digest paths in the project's roots, through the `Fs`
//! port. Every answer names a path as [`Verbs::shown`] prints it, but `temp`'s, which is absolute.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specforge_effects::contained::{TEMP, temp_root};
use specforge_effects::fs::{FileKind, READ_CEILING};
use specforge_effects::{FsChange, FsChanged, FsOp, ListRequest, OutcomeClass, Place, Root};

use super::{Access, Located, Verbs, refused};
use crate::{OpError, OpErrorKind};

mod search;

pub use search::{
    Context, Counted, LINE_CHARS, Match, SEARCH_BYTES, SEARCH_MAX, Search, SearchAnswer,
    SearchMode, search,
};

/// The most entries `list` answers unless asked for another bound.
pub const LIST_MAX: usize = 10_000;

/// One path `stat` asked about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatEntry {
    pub path: String,
    /// The root it was read in.
    pub root: String,
    /// Where it lands, links resolved.
    pub real: String,
    pub name: String,
    pub parent: String,
    pub exists: bool,
    /// None when it does not exist.
    pub kind: Option<FileKind>,
    /// A file's size in bytes.
    pub size: Option<u64>,
    /// Milliseconds since the Unix epoch, when the file system keeps it.
    pub modified_ms: Option<u64>,
}

/// `files stat`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatAnswer {
    pub entries: Vec<StatEntry>,
}

/// One entry `list` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListEntry {
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
    /// A file's lines (see [`lines_of`]), only under `lines`; null when it was not read whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Option<usize>>,
}

/// `files list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListAnswer {
    pub entries: Vec<ListEntry>,
    pub count: usize,
    /// The lines of every file read whole, only under `lines`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<usize>,
    /// It stopped at its bound.
    pub truncated: bool,
}

/// What `files list` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    pub dir: PathBuf,
    pub globs: Vec<String>,
    pub kind: Option<FileKind>,
    pub depth: Option<usize>,
    /// Leave out what `.gitignore` ignores.
    pub ignore: bool,
    pub hidden: bool,
    pub max: usize,
    /// Count each file's lines, and their total.
    pub lines: bool,
}

/// Each of `paths`: its kind, size and modification time, or that it does not exist.
pub fn stat(verbs: &Verbs, paths: &[PathBuf]) -> Result<StatAnswer, OpError> {
    let mut entries = Vec::new();
    for typed in paths {
        let (located, request) = verbs.held(typed, FsOp::Stat)?;
        let shown = verbs.shown(&located.at);
        let outcome = verbs.ports().fs.run(verbs.roots(), &request);
        let stat = match outcome.class {
            OutcomeClass::Ok => outcome.stat,
            OutcomeClass::Missing => None,
            other => return Err(refused(other, &shown)),
        };
        let name = located
            .at
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent = located
            .at
            .parent()
            .map(|parent| verbs.shown(parent))
            .unwrap_or_default();
        entries.push(StatEntry {
            path: shown,
            root: located.root.as_str().to_string(),
            real: located.at.to_string_lossy().replace('\\', "/"),
            name,
            parent,
            exists: stat.is_some(),
            kind: stat.map(|stat| stat.kind),
            // A directory's size is the file system's, not its contents'.
            size: stat
                .filter(|stat| stat.kind == FileKind::File)
                .map(|stat| stat.size),
            modified_ms: stat.and_then(|stat| stat.modified_ms),
        });
    }
    Ok(StatAnswer { entries })
}

/// The globs, checked: `invalid_input` names the one that does not parse (a leading `!` excludes, and is not part of
/// the glob).
fn checked_globs(globs: &[String]) -> Result<(), OpError> {
    for glob in globs {
        globset::Glob::new(glob.strip_prefix('!').unwrap_or(glob)).map_err(|error| {
            OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                format!("the glob {glob:?} does not parse: {error}"),
            )
        })?;
    }
    Ok(())
}

/// The entries under `list.dir`.
pub fn list(verbs: &Verbs, list: &List) -> Result<ListAnswer, OpError> {
    checked_globs(&list.globs)?;
    let (located, request) = verbs.held(
        &list.dir,
        FsOp::Walk {
            glob: None,
            max_entries: list.max.min(u32::MAX as usize) as u32,
        },
    )?;
    let listing = verbs.ports().fs.list(
        verbs.roots(),
        &ListRequest {
            root: request.root,
            path: request.path,
            globs: list.globs.clone(),
            kind: list.kind,
            depth: list.depth,
            ignore: list.ignore,
            hidden: list.hidden,
            max_entries: list.max,
        },
    );
    if listing.class != OutcomeClass::Ok {
        return Err(refused(listing.class, &verbs.shown(&located.at)));
    }
    let mut truncated = listing.truncated;
    let mut budget = SEARCH_BYTES;
    let entries: Vec<ListEntry> = listing
        .entries
        .iter()
        .map(|entry| ListEntry {
            path: verbs.shown_in(&located.root, &entry.path),
            kind: entry.kind,
            size: entry.size,
            lines: (list.lines && entry.kind == FileKind::File).then(|| {
                let (lines, cut) = lines_read(verbs, &located.root, &entry.path, &mut budget);
                truncated |= cut;
                lines
            }),
        })
        .collect();
    Ok(ListAnswer {
        count: entries.len(),
        lines: list.lines.then(|| {
            entries
                .iter()
                .filter_map(|entry| entry.lines.flatten())
                .sum()
        }),
        entries,
        truncated,
    })
}

/// The lines of the file `rel` in `root`, read whole within `budget` (which the read spends), and whether a bound cut
/// it: none, and cut, when the file is larger than one read or past the budget; none, not cut, when it is withheld.
fn lines_read(verbs: &Verbs, root: &Root, rel: &str, budget: &mut u64) -> (Option<usize>, bool) {
    let read = specforge_effects::FsRequest {
        root: root.as_str().to_string(),
        path: rel.to_string(),
        op: FsOp::Read {
            max_bytes: READ_CEILING.min(*budget),
        },
    };
    if verbs.permit_fs(&read).is_err() {
        return (None, false);
    }
    let outcome = verbs.ports().fs.run(verbs.roots(), &read);
    *budget = budget.saturating_sub(outcome.bytes.len() as u64);
    match outcome.class {
        OutcomeClass::Ok if !outcome.truncated => (Some(lines_of(&outcome.bytes)), false),
        OutcomeClass::Ok | OutcomeClass::Capped => (None, true),
        _ => (None, false),
    }
}

/// `files read`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadAnswer {
    pub path: String,
    /// The bytes read as UTF-8, an invalid sequence replaced.
    pub text: String,
    /// How many bytes were read.
    pub bytes: u64,
    /// See [`lines_of`].
    pub lines: usize,
    /// The file is larger than one read ([`READ_CEILING`]).
    pub truncated: bool,
}

/// A file's text, through one capped read: a directory is `invalid_input`, a missing file `file_not_found`, and the
/// host's state or a credential is refused.
pub fn read(verbs: &Verbs, typed: &Path) -> Result<ReadAnswer, OpError> {
    let (located, request) = verbs.held(
        typed,
        FsOp::Read {
            max_bytes: READ_CEILING,
        },
    )?;
    let shown = verbs.shown(&located.at);
    let outcome = verbs.ports().fs.run(verbs.roots(), &request);
    if outcome.class != OutcomeClass::Ok {
        return Err(refused(outcome.class, &shown));
    }
    if outcome.stat.is_some_and(|stat| stat.kind != FileKind::File) {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            format!("{shown} is not a file"),
        ));
    }
    Ok(ReadAnswer {
        path: shown,
        text: String::from_utf8_lossy(&outcome.bytes).into_owned(),
        bytes: outcome.bytes.len() as u64,
        lines: lines_of(&outcome.bytes),
        truncated: outcome.truncated,
    })
}

/// The lines of `bytes`: its `\n`s, and one more for a last line with none, which `wc -l` does not count.
pub fn lines_of(bytes: &[u8]) -> usize {
    let newlines = bytes.iter().filter(|&&byte| byte == b'\n').count();
    newlines + usize::from(bytes.last().is_some_and(|&last| last != b'\n'))
}

/// `files temp`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TempAnswer {
    /// Absolute, as a shell gives it to the next command.
    pub path: String,
}

/// What `files temp` asks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Temp {
    /// A directory, not a file.
    pub dir: bool,
    pub prefix: String,
    pub suffix: String,
}

/// `files mkdir`, `copy`, `move` and `remove`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeAnswer {
    /// Each path made, written or removed.
    pub changed: Vec<String>,
}

/// One digest `files digest` answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Digested {
    /// The file; none for `--text`.
    pub path: Option<String>,
    /// Lowercase hex.
    pub sha256: String,
    pub bytes: u64,
}

/// `files digest`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DigestAnswer {
    pub digests: Vec<Digested>,
}

/// A new empty file (`0600`), or directory (`0700`), with a name no other has, in the `temp` root, made on first use.
pub fn temp(verbs: Verbs, temp: &Temp) -> Result<TempAnswer, OpError> {
    for (what, part) in [("prefix", &temp.prefix), ("suffix", &temp.suffix)] {
        if part.contains(['/', '\\']) || part.contains("..") || part.chars().any(char::is_control) {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                format!("the name's {what} {part:?} takes no separator and no `..`"),
            ));
        }
    }
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|error| {
        OpError::new(
            OpErrorKind::Internal,
            OpErrorKind::Internal.as_str(),
            format!("no random name: {error}"),
        )
    })?;
    let hex: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("{}{hex}{}", temp.prefix, temp.suffix);
    let at = Place {
        root: TEMP.to_string(),
        path: name.clone(),
    };
    verbs.permit_write(&temp_root(), &name)?;
    // Made once permitted: a refused `temp` makes nothing.
    let (verbs, root) = verbs.make_temp_root()?;
    changed(
        &verbs,
        verbs
            .ports()
            .fs
            .change(verbs.roots(), &FsChange::Create { at, dir: temp.dir }),
    )?;
    Ok(TempAnswer {
        path: root.join(name).to_string_lossy().into_owned(),
    })
}

/// Each of `paths` made, with its missing parents.
pub fn mkdir(verbs: &Verbs, paths: &[PathBuf]) -> Result<ChangeAnswer, OpError> {
    let mut answer = ChangeAnswer {
        changed: Vec::new(),
    };
    for typed in paths {
        let at = written(verbs, typed)?;
        answer.changed.extend(changed(
            verbs,
            verbs
                .ports()
                .fs
                .change(verbs.roots(), &FsChange::Mkdir { at }),
        )?);
    }
    Ok(answer)
}

/// `from` copied onto `to`, or into it when it is a directory; a directory with what it holds.
pub fn copy(verbs: &Verbs, from: &Path, to: &Path) -> Result<ChangeAnswer, OpError> {
    let source = verbs.located(from, &Access::Fs(FsOp::Read { max_bytes: 0 }))?;
    let change = FsChange::Copy {
        from: place(&source),
        to: written(verbs, to)?,
    };
    Ok(ChangeAnswer {
        changed: changed(verbs, verbs.ports().fs.change(verbs.roots(), &change))?,
    })
}

/// `from` moved onto `to`, or into it when it is a directory: renamed in one device, copied then removed across two.
pub fn move_to(verbs: &Verbs, from: &Path, to: &Path) -> Result<ChangeAnswer, OpError> {
    let change = FsChange::Move {
        from: written(verbs, from)?,
        to: written(verbs, to)?,
    };
    Ok(ChangeAnswer {
        changed: changed(verbs, verbs.ports().fs.change(verbs.roots(), &change))?,
    })
}

/// Each of `paths` removed: a file, an empty directory, or with `recursive` a directory and what it holds. A root, or
/// a directory that holds one, is never removed.
/// With `force`, a path that does not exist is skipped.
pub fn remove(
    verbs: &Verbs,
    paths: &[PathBuf],
    recursive: bool,
    force: bool,
) -> Result<ChangeAnswer, OpError> {
    // Every path is held before any is removed: a refused one changes nothing.
    let places: Vec<Place> = paths
        .iter()
        .map(|typed| written(verbs, typed))
        .collect::<Result<_, _>>()?;
    let mut answer = ChangeAnswer {
        changed: Vec::new(),
    };
    for at in places {
        let outcome = verbs
            .ports()
            .fs
            .change(verbs.roots(), &FsChange::Remove { at, recursive });
        if force && outcome.class == OutcomeClass::Missing {
            continue;
        }
        answer.changed.extend(changed(verbs, outcome)?);
    }
    Ok(answer)
}

/// The SHA-256 of each file of `paths`, then of `text` when there is one.
pub fn digest(
    verbs: &Verbs,
    paths: &[PathBuf],
    text: Option<&str>,
) -> Result<DigestAnswer, OpError> {
    let mut digests = Vec::new();
    for typed in paths {
        let (located, request) = verbs.held(typed, FsOp::Stat)?;
        let shown = verbs.shown(&located.at);
        let stat = verbs.ports().fs.run(verbs.roots(), &request);
        if stat.stat.is_some_and(|stat| stat.kind != FileKind::File) {
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                format!("{shown} is not a file"),
            ));
        }
        let located = verbs.located(typed, &Access::Fs(FsOp::Read { max_bytes: 0 }))?;
        let digest = verbs.ports().fs.digest(verbs.roots(), &place(&located));
        if digest.class != OutcomeClass::Ok {
            return Err(refused(digest.class, &shown));
        }
        digests.push(Digested {
            path: Some(shown),
            sha256: digest.sha256,
            bytes: digest.bytes,
        });
    }
    if let Some(text) = text {
        digests.push(Digested {
            path: None,
            sha256: specforge_package::Sha256::of(text.as_bytes()).to_string(),
            bytes: text.len() as u64,
        });
    }
    Ok(DigestAnswer { digests })
}

/// `typed`, held for a write.
fn written(verbs: &Verbs, typed: &Path) -> Result<Place, OpError> {
    verbs
        .located(typed, &Access::Write)
        .map(|located| place(&located))
}

fn place(located: &Located) -> Place {
    Place {
        root: located.root.as_str().to_string(),
        path: located.rel.clone(),
    }
}

/// What a change made, as an answer prints it, or its refusal.
fn changed(verbs: &Verbs, outcome: FsChanged) -> Result<Vec<String>, OpError> {
    let detail = outcome.detail.unwrap_or_default();
    match outcome.class {
        OutcomeClass::Ok => Ok(outcome
            .changed
            .iter()
            .map(|place| verbs.shown_named(&place.root, &place.path))
            .collect()),
        OutcomeClass::Outside => Err(OpError::new(
            OpErrorKind::PermissionDenied,
            crate::OUTSIDE_ROOT,
            detail,
        )),
        OutcomeClass::Missing => Err(OpError::new(
            OpErrorKind::FileNotFound,
            OpErrorKind::FileNotFound.as_str(),
            detail,
        )),
        _ => Err(OpError::new(
            OpErrorKind::PermissionDenied,
            OpErrorKind::PermissionDenied.as_str(),
            detail,
        )),
    }
}
