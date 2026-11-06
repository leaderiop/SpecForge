//! `specforge files search` (ADR 0339's amendment): ripgrep's own engine, a `grep-regex` matcher and a `grep-searcher`
//! searcher, over the bytes the `Fs` port read.

use std::path::PathBuf;

use grep_matcher::{LineTerminator, Matcher as _};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{
    BinaryDetection, Searcher, SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkMatch,
};
use serde::Serialize;
use specforge_effects::fs::{FILE_CEILING, FileKind};
use specforge_effects::{FsOp, ListRequest, OutcomeClass, Root};

use super::{LIST_MAX, checked_globs};
use crate::verbs::{Verbs, refused};
use crate::{OpError, OpErrorKind};

/// The most matches `search` answers unless asked for another bound.
pub const SEARCH_MAX: usize = 1_000;
/// The most bytes one search reads, over every file.
pub const SEARCH_BYTES: u64 = 64 * 1024 * 1024;
/// The path an answer names standard input by.
pub const STDIN: &str = "<stdin>";
/// The most characters of a matching line an answer keeps.
pub const LINE_CHARS: usize = 500;

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

/// One file's count of matching lines, under [`SearchMode::Counts`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Counted {
    pub path: String,
    pub count: usize,
}

/// Why a file was left out of a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreadWhy {
    /// The host or the system refused it: the host's own state, a credential, a file with more than one name, a mode.
    Denied,
    /// The project's effects ceiling leaves it out.
    Ceiling,
    /// Larger than one file's or standard input's bound (64 MiB).
    TooLarge,
    /// The search's budget (64 MiB over every file) ran out at or before it: what was read of it was searched.
    PastBudget,
    /// A binary file typed by name, as git decides one. A walked one is skipped, as `grep -I`, and not unread.
    Binary,
    /// A pipe, a device or a socket typed by name.
    NotAFile,
    /// The walk stopped at its bound in this directory.
    WalkBound,
    /// The system would not let it be listed or read to its end: a directory, or standard input.
    Unreadable,
}

impl UnreadWhy {
    /// What an answer says, and what a person reads.
    pub const fn name(self) -> &'static str {
        match self {
            UnreadWhy::Denied => "denied",
            UnreadWhy::Ceiling => "ceiling",
            UnreadWhy::TooLarge => "too_large",
            UnreadWhy::PastBudget => "past_budget",
            UnreadWhy::Binary => "binary",
            UnreadWhy::NotAFile => "not_a_file",
            UnreadWhy::WalkBound => "walk_bound",
            UnreadWhy::Unreadable => "unreadable",
        }
    }
}

impl Serialize for UnreadWhy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

/// What a search did not read, or did not read whole. A verdict never passes on one (ADR 0339 D4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unread {
    pub path: String,
    pub why: UnreadWhy,
}

/// Standard input as the caller read it: only when `-` is a path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stdin {
    pub bytes: Vec<u8>,
    /// Why `bytes` are not all of it.
    pub cut: Option<UnreadWhy>,
}

/// What `files search` answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchMode {
    /// Each matching line.
    #[default]
    Lines,
    /// The files that match, each at its first match (`-l`).
    Files,
    /// Each file's count of matching lines: every file with a match, and every file typed by name (`--counts`).
    Counts,
}

/// `files search`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchAnswer {
    /// Each matching line; empty under `-l` and `--counts`.
    pub matches: Vec<Match>,
    /// The files that match; only under `-l`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    /// Each file's count; only under `--counts`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<Vec<Counted>>,
    /// The length of the list the mode answers.
    pub count: usize,
    /// It stopped at its bound, or some bytes were past one: a file or standard input read in part or not at all.
    pub truncated: bool,
    /// Each typed path that does not exist; the others were searched.
    pub missing: Vec<String>,
    /// What it did not read, or not whole.
    pub unread: Vec<Unread>,
}

/// What `files search` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Search {
    pub pattern: String,
    pub paths: Vec<PathBuf>,
    pub globs: Vec<String>,
    /// Leave out what `.gitignore` ignores, as `list` does.
    pub ignore: bool,
    /// Walk names that start with `.`.
    pub hidden: bool,
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
    pub mode: SearchMode,
    /// The most matches, files or counts answered.
    pub max: usize,
    /// Standard input, searched for the path `-` as [`STDIN`].
    pub stdin: Stdin,
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
    let searcher = SearcherBuilder::new()
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
        files: (search.mode == SearchMode::Files).then(Vec::new),
        counts: (search.mode == SearchMode::Counts).then(Vec::new),
        count: 0,
        truncated: false,
        missing: Vec::new(),
        unread: Vec::new(),
    };
    let mut budget = SEARCH_BYTES;
    let typed: Vec<PathBuf> = if search.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        search.paths.clone()
    };
    let mut engine = Engine {
        searcher,
        matcher,
        and_not,
        search,
    };
    for path in &typed {
        if path.as_os_str() == "-" {
            if let Some(why) = search.stdin.cut {
                answer.truncated = true;
                answer.unread(STDIN.to_string(), why);
            }
            engine.judged(&mut answer, STDIN.to_string(), &search.stdin.bytes, true)?;
            continue;
        }
        let (located, request) = verbs.held(path, FsOp::Stat)?;
        let outcome = verbs.ports().fs.run(verbs.roots(), &request);
        // Each file to search, and whether it was typed by name.
        let files: Vec<(Root, String, bool)> =
            match (outcome.class, outcome.stat.map(|stat| stat.kind)) {
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
                            ignore: search.ignore,
                            hidden: search.hidden,
                            max_entries: LIST_MAX,
                        },
                    );
                    if listing.class != OutcomeClass::Ok {
                        return Err(refused(listing.class, &verbs.shown(&located.at)));
                    }
                    if listing.truncated {
                        answer.truncated = true;
                        answer.unread(verbs.shown(&located.at), UnreadWhy::WalkBound);
                    }
                    for rel in &listing.unread {
                        let shown = verbs.shown_in(&walked.root, rel);
                        answer.unread(shown, UnreadWhy::Unreadable);
                    }
                    listing
                        .entries
                        .into_iter()
                        .map(|entry| (walked.root.clone(), entry.path, false))
                        .collect()
                }
                (OutcomeClass::Ok, _) => vec![(located.root.clone(), request.path.clone(), true)],
                (OutcomeClass::Missing, _) => {
                    answer.missing.push(verbs.shown(&located.at));
                    continue;
                }
                (class, _) => return Err(refused(class, &verbs.shown(&located.at))),
            };
        for (root, rel, named) in files {
            if answer.listed() >= search.max {
                answer.truncated = true;
                break;
            }
            let shown = verbs.shown_in(&root, &rel);
            if budget == 0 {
                answer.truncated = true;
                answer.unread(shown, UnreadWhy::PastBudget);
                continue;
            }
            let read = specforge_effects::FsRequest {
                root: root.as_str().to_string(),
                path: rel.clone(),
                op: FsOp::Read {
                    max_bytes: FILE_CEILING.min(budget),
                },
            };
            // What a walk cannot read is listed as unread; a file typed by name is refused, so no verdict is judged on
            // a file it did not read.
            if let Err(error) = verbs.permit_fs(&read) {
                if named {
                    return Err(error);
                }
                answer.unread(shown, UnreadWhy::Ceiling);
                continue;
            }
            let outcome = verbs.ports().fs.read_whole(verbs.roots(), &read);
            match outcome.class {
                OutcomeClass::Ok => {}
                class if named => return Err(refused(class, &shown)),
                // Gone since the walk: nothing is left to read.
                OutcomeClass::Missing => continue,
                OutcomeClass::Capped => {
                    answer.truncated = true;
                    answer.unread(shown, UnreadWhy::TooLarge);
                    continue;
                }
                _ => {
                    answer.unread(shown, UnreadWhy::Denied);
                    continue;
                }
            }
            if outcome.stat.is_some_and(|stat| stat.kind != FileKind::File) {
                answer.unread(shown, UnreadWhy::NotAFile);
                continue;
            }
            budget = budget.saturating_sub(outcome.bytes.len() as u64);
            if outcome.truncated {
                answer.truncated = true;
                answer.unread(shown.clone(), UnreadWhy::PastBudget);
            }
            engine.judged(&mut answer, shown, &outcome.bytes, named)?;
        }
    }
    answer.count = answer.listed();
    Ok(answer)
}

/// What searches each file: the library's searcher and matchers, and the search they were built for.
struct Engine<'a> {
    searcher: Searcher,
    matcher: RegexMatcher,
    and_not: Vec<RegexMatcher>,
    search: &'a Search,
}

impl Engine<'_> {
    /// [`Self::run`] unless `bytes` are binary, as git decides (a NUL in the first 8000): a walked one is skipped, as
    /// `grep -I`, and one typed by name is unread, and counted at 0.
    fn judged(
        &mut self,
        answer: &mut SearchAnswer,
        shown: String,
        bytes: &[u8],
        named: bool,
    ) -> Result<(), OpError> {
        if !specforge_effects::git::is_binary(bytes) {
            return self.run(answer, shown, bytes, named);
        }
        if named {
            answer.unread(shown.clone(), UnreadWhy::Binary);
            return self.run(answer, shown, b"", named);
        }
        Ok(())
    }

    /// `bytes`, shown as `shown`, searched into `answer`: its matches, or under `-l` and `--counts` its name and count
    /// (a file typed by name is counted even at 0).
    fn run(
        &mut self,
        answer: &mut SearchAnswer,
        shown: String,
        bytes: &[u8],
        named: bool,
    ) -> Result<(), OpError> {
        let mut found = Found {
            matcher: &self.matcher,
            and_not: &self.and_not,
            search: self.search,
            path: &shown,
            answer,
            before: Vec::new(),
            last: None,
            kept: 0,
        };
        self.searcher
            .search_slice(&self.matcher, bytes, &mut found)
            .map_err(|error| {
                OpError::new(
                    OpErrorKind::Internal,
                    OpErrorKind::Internal.as_str(),
                    format!("the search of {shown} stopped: {error}"),
                )
            })?;
        let kept = found.kept;
        match (&mut answer.files, &mut answer.counts) {
            (Some(files), _) if kept > 0 => files.push(shown),
            (_, Some(counts)) if kept > 0 || named => counts.push(Counted {
                path: shown,
                count: kept,
            }),
            _ => {}
        }
        Ok(())
    }
}

impl SearchAnswer {
    fn unread(&mut self, path: String, why: UnreadWhy) {
        self.unread.push(Unread { path, why });
    }

    /// The length of the list the mode answers.
    fn listed(&self) -> usize {
        match (&self.files, &self.counts) {
            (Some(files), _) => files.len(),
            (_, Some(counts)) => counts.len(),
            _ => self.matches.len(),
        }
    }
}

/// `pattern` (the search's own or an `--and-not`) as `search` asks it matched, as ripgrep builds its matcher: a line's
/// `^` and `$` at each line, CRLF lines read without their `\r`, and unless `multiline` no match that crosses a line (a
/// pattern that needs a `\n` is refused).
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
    /// The matches kept in this file.
    kept: usize,
}

impl Sink for Found<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _: &Searcher, found: &SinkMatch<'_>) -> Result<bool, Self::Error> {
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
        match self.search.mode {
            // A file's first match names it.
            SearchMode::Files => {
                self.kept += 1;
                return Ok(false);
            }
            SearchMode::Counts => {
                self.kept += 1;
                return Ok(true);
            }
            SearchMode::Lines if self.answer.matches.len() >= self.search.max => {
                self.answer.truncated = true;
                return Ok(false);
            }
            SearchMode::Lines => self.kept += 1,
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
