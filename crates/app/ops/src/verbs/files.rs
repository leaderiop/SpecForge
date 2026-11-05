//! `specforge files` (ADR 0339): stat, list, search, change and digest paths in the project's roots, through the `Fs`
//! port. Every answer names a path as [`Verbs::shown`] prints it, but `temp`'s, which is absolute.

use std::path::{Path, PathBuf};

use grep_matcher::{LineTerminator, Matcher as _};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{
    BinaryDetection, Searcher, SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkMatch,
};
use serde::Serialize;
use specforge_effects::contained::{TEMP, temp_root};
use specforge_effects::fs::{FileKind, READ_CEILING};
use specforge_effects::{FsChange, FsChanged, FsOp, ListRequest, OutcomeClass, Place, Root};

use super::{Access, Located, Verbs, refused};
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

/// One line `search` matched, or under `multiline` the lines one match spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    pub path: String,
    /// 1-based.
    pub line: usize,
    /// The last line a multiline match spans; only under `multiline`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// 1-based, in characters; 1 for an inverted line.
    pub column: usize,
    /// The line (the lines, joined by `\n`), at most [`LINE_CHARS`] characters.
    pub text: String,
    /// The lines before it; only when context is asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Vec<Context>>,
    /// The lines after it; only when context is asked. A line between two close matches is the earlier's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<Context>>,
}

/// A line around a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Context {
    /// 1-based.
    pub line: usize,
    /// At most [`LINE_CHARS`] characters.
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
    /// The pattern is a literal, not a regex.
    pub fixed: bool,
    /// The lines that do not match.
    pub invert: bool,
    /// A match may span lines: the pattern may hold `\n`.
    pub multiline: bool,
    /// Lines answered before each match.
    pub before: usize,
    /// Lines answered after each match.
    pub after: usize,
    /// A reported line (or multiline match) one of these matches is left out; context is kept.
    pub and_not: Vec<String>,
    pub max: usize,
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
    let entries: Vec<ListEntry> = listing
        .entries
        .iter()
        .map(|entry| ListEntry {
            path: verbs.shown_in(&located.root, &entry.path),
            kind: entry.kind,
            size: entry.size,
        })
        .collect();
    Ok(ListAnswer {
        count: entries.len(),
        entries,
        truncated: listing.truncated,
    })
}

/// The lines under `search.paths` that match `search.pattern`: each line once, at its first match.
///
/// ripgrep's own engine runs on the bytes the `Fs` port read (ADR 0339's amendment): a `grep-regex` matcher and a
/// `grep-searcher` searcher over the slice, never over a path, which would open a file past the port.
pub fn search(verbs: &Verbs, search: &Search) -> Result<SearchAnswer, OpError> {
    checked_globs(&search.globs)?;
    let and_not = search
        .and_not
        .iter()
        .map(|pattern| matcher(search, pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let matcher = matcher(search, &search.pattern)?;
    let mut searcher = SearcherBuilder::new()
        .line_number(true)
        .line_terminator(LineTerminator::crlf())
        .invert_match(search.invert)
        .multi_line(search.multiline)
        .before_context(search.before)
        .after_context(search.after)
        // A binary file is skipped before it is searched, by git's rule.
        .binary_detection(BinaryDetection::none())
        .build();
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
            // A binary file, as git decides one (a NUL in its first 8000 bytes), is not searched.
            if specforge_effects::git::is_binary(&outcome.bytes) {
                continue;
            }
            let shown = verbs.shown_in(&root, &rel);
            let mut found = Found {
                matcher: &matcher,
                and_not: &and_not,
                search,
                path: &shown,
                answer: &mut answer,
                before: Vec::new(),
                last: None,
            };
            searcher
                .search_slice(&matcher, &outcome.bytes, &mut found)
                .map_err(|error| {
                    OpError::new(
                        OpErrorKind::Internal,
                        OpErrorKind::Internal.as_str(),
                        format!("the search of {shown} stopped: {error}"),
                    )
                })?;
        }
    }
    answer.count = answer.matches.len();
    Ok(answer)
}

/// `pattern` (the search's own or an `--and-not`) as `search` asks it matched, as ripgrep builds its matcher: a line's `^` and `$` at each line, CRLF lines
/// read without their `\r`, and unless `multiline` no match that crosses a line (a pattern that needs a `\n` is
/// refused).
fn matcher(search: &Search, pattern: &str) -> Result<RegexMatcher, OpError> {
    let mut builder = RegexMatcherBuilder::new();
    builder
        .case_insensitive(search.ignore_case)
        .fixed_strings(search.fixed)
        .multi_line(true)
        .crlf(true);
    if search.multiline {
        builder.line_terminator(None);
    }
    builder.build(pattern).map_err(|error| {
        OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            format!("the pattern does not parse: {error}"),
        )
    })
}

/// The answer one file's search adds to: each match the searcher reports, with its context, until the answer holds
/// `max`.
struct Found<'a> {
    matcher: &'a RegexMatcher,
    /// A match one of these matches is left out.
    and_not: &'a [RegexMatcher],
    search: &'a Search,
    path: &'a str,
    answer: &'a mut SearchAnswer,
    /// The context lines reported since the last match: the next match's `before`.
    before: Vec<Context>,
    /// The index of this file's last match in the answer: an `after` line is its.
    last: Option<usize>,
}

impl Sink for Found<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _: &Searcher, found: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        if self.answer.matches.len() >= self.search.max {
            self.answer.truncated = true;
            return Ok(false);
        }
        let bytes = found.bytes();
        let text = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let text = text.strip_suffix(b"\r").unwrap_or(text);
        if self
            .and_not
            .iter()
            .any(|matcher| matcher.is_match(text).unwrap_or(false))
        {
            // Its context goes with it: the lines before are no other match's, those after the next one's before.
            self.before.clear();
            self.last = None;
            return Ok(true);
        }
        let line = found.line_number().unwrap_or(0) as usize;
        let column = match self.matcher.find(bytes) {
            Ok(Some(at)) => {
                String::from_utf8_lossy(&bytes[..at.start()])
                    .chars()
                    .count()
                    + 1
            }
            _ => 1,
        };
        let context = self.search.before > 0 || self.search.after > 0;
        self.answer.matches.push(Match {
            path: self.path.to_string(),
            line,
            end_line: self
                .search
                .multiline
                .then(|| line + found.lines().count().saturating_sub(1)),
            column,
            text: lines_text(bytes),
            before: context.then(|| std::mem::take(&mut self.before)),
            after: context.then(Vec::new),
        });
        self.last = Some(self.answer.matches.len() - 1);
        Ok(true)
    }

    fn context(&mut self, _: &Searcher, context: &SinkContext<'_>) -> Result<bool, Self::Error> {
        let line = Context {
            line: context.line_number().unwrap_or(0) as usize,
            text: lines_text(context.bytes()),
        };
        match (context.kind(), self.last) {
            (SinkContextKind::After, Some(last)) => {
                if let Some(after) = self.answer.matches[last].after.as_mut() {
                    after.push(line);
                }
            }
            _ => self.before.push(line),
        }
        Ok(true)
    }

    fn context_break(&mut self, _: &Searcher) -> Result<bool, Self::Error> {
        // Lines past a break are not next to the next match.
        self.before.clear();
        Ok(true)
    }
}

/// The line or lines the searcher reported, each without its terminator, joined by `\n`, at most [`LINE_CHARS`]
/// characters.
fn lines_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .chars()
        .take(LINE_CHARS)
        .collect()
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
