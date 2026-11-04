//! `specforge files` (ADR 0339): stat, list and search paths in the project's roots, through the `Fs` port. Every
//! answer names a path as [`Verbs::shown`] prints it.

use std::path::PathBuf;

use serde::Serialize;
use specforge_effects::fs::{FileKind, READ_CEILING};
use specforge_effects::{FsOp, ListRequest, OutcomeClass, Root};

use super::{Verbs, refused};
use crate::{OpError, OpErrorKind};

/// The most entries `list` answers unless asked for another bound.
pub const LIST_MAX: usize = 10_000;
/// The most matches `search` answers unless asked for another bound.
pub const SEARCH_MAX: usize = 1_000;
/// The most bytes one search reads, over every file.
pub const SEARCH_BYTES: u64 = 64 * 1024 * 1024;
/// The most characters of a matching line an answer keeps.
pub const LINE_CHARS: usize = 500;

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
    /// `file`, `dir` or `other`; none when it does not exist.
    pub kind: Option<&'static str>,
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
    pub kind: &'static str,
    pub size: u64,
}

/// `files list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListAnswer {
    pub entries: Vec<ListEntry>,
    pub count: usize,
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
}

/// One line `search` matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    pub path: String,
    /// 1-based.
    pub line: usize,
    /// 1-based, in characters.
    pub column: usize,
    /// The line, at most [`LINE_CHARS`] characters.
    pub text: String,
}

/// `files search`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchAnswer {
    pub matches: Vec<Match>,
    pub count: usize,
    /// It stopped at its bound, or a file was larger than one read.
    pub truncated: bool,
}

/// What `files search` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    pub pattern: String,
    pub paths: Vec<PathBuf>,
    pub globs: Vec<String>,
    pub ignore_case: bool,
    pub max: usize,
}

fn kind_name(kind: FileKind) -> &'static str {
    match kind {
        FileKind::File => "file",
        FileKind::Dir => "dir",
        FileKind::Other => "other",
    }
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
            kind: stat.map(|stat| kind_name(stat.kind)),
            // A directory's size is the file system's, not its contents'.
            size: stat
                .filter(|stat| stat.kind == FileKind::File)
                .map(|stat| stat.size),
            modified_ms: stat.and_then(|stat| stat.modified_ms),
        });
    }
    Ok(StatAnswer { entries })
}

/// The globs, checked: `invalid_input` names the one that does not parse.
fn checked_globs(globs: &[String]) -> Result<(), OpError> {
    for glob in globs {
        globset::Glob::new(glob).map_err(|error| {
            OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                format!("--glob {glob:?} does not parse: {error}"),
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
    let entries: Vec<ListEntry> = listing
        .entries
        .iter()
        .map(|entry| ListEntry {
            path: verbs.shown_in(&located.root, &entry.path),
            kind: kind_name(entry.kind),
            size: entry.size,
        })
        .collect();
    Ok(ListAnswer {
        count: entries.len(),
        entries,
        truncated: listing.truncated,
    })
}

/// The lines under `search.paths` that match `search.pattern`, the first match of each line.
pub fn search(verbs: &Verbs, search: &Search) -> Result<SearchAnswer, OpError> {
    checked_globs(&search.globs)?;
    let pattern = regex::RegexBuilder::new(&search.pattern)
        .case_insensitive(search.ignore_case)
        .build()
        .map_err(|error| {
            OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                format!("the pattern does not parse: {error}"),
            )
        })?;
    let mut answer = SearchAnswer {
        matches: Vec::new(),
        count: 0,
        truncated: false,
    };
    let mut budget = SEARCH_BYTES;
    let typed: Vec<PathBuf> = if search.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        search.paths.clone()
    };
    for path in &typed {
        let (located, request) = verbs.held(path, FsOp::Stat)?;
        let outcome = verbs.ports().fs.run(verbs.roots(), &request);
        let files: Vec<(Root, String)> = match (outcome.class, outcome.stat.map(|stat| stat.kind)) {
            (OutcomeClass::Ok, Some(FileKind::Dir)) => {
                let (walked, walk) = verbs.held(
                    path,
                    FsOp::Walk {
                        glob: None,
                        max_entries: LIST_MAX as u32,
                    },
                )?;
                let listing = verbs.ports().fs.list(
                    verbs.roots(),
                    &ListRequest {
                        root: walk.root,
                        path: walk.path,
                        globs: search.globs.clone(),
                        kind: Some(FileKind::File),
                        depth: None,
                        ignore: true,
                        hidden: false,
                        max_entries: LIST_MAX,
                    },
                );
                answer.truncated |= listing.truncated;
                listing
                    .entries
                    .into_iter()
                    .map(|entry| (walked.root.clone(), entry.path))
                    .collect()
            }
            (OutcomeClass::Ok, _) => vec![(located.root.clone(), request.path.clone())],
            (class, _) => return Err(refused(class, &verbs.shown(&located.at))),
        };
        for (root, rel) in files {
            if answer.matches.len() >= search.max || budget == 0 {
                answer.truncated = true;
                break;
            }
            let read = specforge_effects::FsRequest {
                root: root.as_str().to_string(),
                path: rel.clone(),
                op: FsOp::Read {
                    max_bytes: READ_CEILING.min(budget),
                },
            };
            // A withheld file or one the ceiling leaves unread is not searched.
            if verbs.permit_fs(&read).is_err() {
                continue;
            }
            let outcome = verbs.ports().fs.run(verbs.roots(), &read);
            if outcome.class != OutcomeClass::Ok {
                continue;
            }
            budget = budget.saturating_sub(outcome.bytes.len() as u64);
            answer.truncated |= outcome.truncated;
            // A file with a NUL in its first 8 KiB is binary, and not searched.
            if outcome.bytes.iter().take(8 * 1024).any(|b| *b == 0) {
                continue;
            }
            let text = String::from_utf8_lossy(&outcome.bytes);
            let shown = verbs.shown_in(&root, &rel);
            for (at, line) in text.lines().enumerate() {
                let Some(found) = pattern.find(line) else {
                    continue;
                };
                if answer.matches.len() >= search.max {
                    answer.truncated = true;
                    break;
                }
                answer.matches.push(Match {
                    path: shown.clone(),
                    line: at + 1,
                    column: line[..found.start()].chars().count() + 1,
                    text: line.chars().take(LINE_CHARS).collect(),
                });
            }
        }
    }
    answer.count = answer.matches.len();
    Ok(answer)
}
