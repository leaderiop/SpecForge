//! `specforge wait` (ADR 0342): a wait for a URL to answer, a file to be written or a line to match, under the user
//! grant, polled through the Http and Fs ports until a condition holds or its time is up. Every condition is judged before any time passes; then it is checked at once,
//! after a pause that doubles to a bound, and once more at the deadline. With no condition it is a delay.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use specforge_effects::fs::{FileKind, FsStat};
use specforge_effects::http::{host_of, scheme};
use specforge_effects::{EffectRequest, EffectsMode, FsOp, OutcomeClass, UnixMillis};
use specforge_protocol_types::HttpRequest;

use super::files::{self, Reach, Search};
use super::http::{self, Ask, Sent};
use super::{Answered, Effect, Located, Verbs, refused};
use crate::{OpError, OpErrorKind};

/// The longest wait, as long as `run`'s longest deadline.
pub const LONGEST: Duration = specforge_effects::process::USER_CEILING;
/// The time a `Retry-After` value asks to wait (RFC 9110 §10.2.3), counted from `now` and held to [`LONGEST`]:
/// delay-seconds (a fraction taken to the millisecond above), or an HTTP-date in any of its three forms, one already
/// past asking for none. `Err` names a value that is neither.
pub fn retry_after(value: &str, now: std::time::SystemTime) -> Result<Duration, String> {
    let value = value.trim();
    let seconds = !value.is_empty()
        && value.chars().all(|c| c.is_ascii_digit() || c == '.')
        && value.matches('.').count() <= 1;
    let asked = match (seconds, value.parse::<f64>()) {
        (true, Ok(seconds)) => {
            let millis = (seconds.min(LONGEST.as_secs_f64()) * 1000.0).ceil();
            Duration::from_millis(millis as u64)
        }
        _ => httpdate::parse_http_date(value)
            .map(|at| at.duration_since(now).unwrap_or_default())
            .map_err(|_| {
                format!(
                    "{value:?} is no Retry-After value: write seconds (120) or an HTTP date (Sun, 06 Nov 1994 \
                     08:49:37 GMT)"
                )
            })?,
    };
    Ok(asked.min(LONGEST))
}

/// The longest a URL probe may take: a short request's time. A probe takes the time the wait has left, held between
/// the poll's first pause and this.
const PROBE_LONGEST: Duration = Duration::from_secs(10);

/// What `specforge wait` asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    /// How long, at most [`LONGEST`].
    pub for_: Duration,
    /// Holds once the URL answers, any status.
    pub url: Option<String>,
    /// Holds once a file the glob names is written.
    pub changed: Option<Changed>,
    /// Holds once a line of the file matches.
    pub matching: Option<Matching>,
    /// `online`, `auto` and `cached` probe live; `replay` answers each probe from the journal.
    pub mode: EffectsMode,
}

/// `--changed`: a glob, from the project's root, and the instant a write counts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub glob: String,
    /// None: the wait's own start.
    pub since: Option<UnixMillis>,
}

/// `--match <pattern> --in <file>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matching {
    /// As `files search` reads one.
    pub pattern: String,
    /// From the current directory.
    pub file: PathBuf,
}

/// Why a wait ended, written as its [`Reason::name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The URL answered.
    Answered,
    /// A file the glob names was written.
    Changed,
    /// A line matched.
    Matched,
    /// Its time ran out, and it had no condition: a delay.
    Elapsed,
    /// Its time ran out before a condition held (exit 124).
    Deadline,
}

impl Reason {
    /// The word the answer writes.
    pub const fn name(self) -> &'static str {
        match self {
            Reason::Answered => "answered",
            Reason::Changed => "changed",
            Reason::Matched => "matched",
            Reason::Elapsed => "elapsed",
            Reason::Deadline => "deadline",
        }
    }

    /// The exit a wait that ended for this reason passes through: 124 when its time ran out first, as `timeout`'s.
    pub const fn exit(self) -> i32 {
        match self {
            Reason::Deadline => 124,
            Reason::Answered | Reason::Changed | Reason::Matched | Reason::Elapsed => 0,
        }
    }
}

impl Serialize for Reason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

/// The last URL probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Probe {
    /// None when no server answered.
    pub status: Option<u16>,
    /// A 2xx answer.
    pub ok: bool,
    /// The class the effects journal writes: `ok`, `status:<code>`, `capped` (a body past 1 MiB), or why no server
    /// answered (`unreachable:connect`).
    pub outcome: OutcomeClass,
}

/// `specforge wait`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaitAnswer {
    pub reason: Reason,
    /// When it started: the next wait's `--since`.
    pub started_at: UnixMillis,
    pub waited_ms: u64,
    pub for_ms: u64,
    /// How many times its conditions were checked.
    pub polls: u64,
    /// With a URL: its last probe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Probe>,
    /// When a file was written: the files written since `--since`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<Vec<String>>,
    /// When a line matched: the first.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<Matched>,
}

/// The line that ended a wait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Matched {
    pub path: String,
    pub line: usize,
    /// At most [`files::LINE_CHARS`] characters.
    pub text: String,
}

/// How a wait's pauses grow: the first, doubling to `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poll {
    pub first: Duration,
    pub max: Duration,
}

impl Poll {
    /// 100 ms, doubling to 1 s.
    pub const BACK_OFF: Poll = Poll {
        first: Duration::from_millis(100),
        max: Duration::from_secs(1),
    };
}

/// A wait's timer: how long since it began, and a pause. A test passes its own, and waits no real time.
pub trait Timer {
    fn elapsed(&self) -> Duration;
    fn pause(&self, span: Duration);
}

/// The machine's monotonic clock, timing from when it was made.
#[derive(Debug, Clone, Copy)]
pub struct MonotonicTimer(Instant);

impl MonotonicTimer {
    pub fn start() -> Self {
        MonotonicTimer(Instant::now())
    }
}

impl Timer for MonotonicTimer {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn pause(&self, span: Duration) {
        std::thread::sleep(span);
    }
}

/// `wait`, judged first, then polled by `poll` on `timer`.
pub fn wait(
    verbs: &Verbs,
    wait: &Wait,
    poll: Poll,
    timer: &dyn Timer,
) -> Result<Answered<WaitAnswer>, OpError> {
    let begun = timer.elapsed();
    let started_at = UnixMillis::now_or_end();
    if let Some(url) = &wait.url {
        judge_url(verbs, url)?;
    }
    let mut watched = wait
        .changed
        .as_ref()
        .map(|changed| Watched::judged(verbs, changed, started_at))
        .transpose()?;
    let searching = wait
        .matching
        .as_ref()
        .map(|matching| searched(verbs, matching))
        .transpose()?;
    let watches_files = watched.is_some() || searching.is_some();
    let mut url = Probing::new(verbs, wait);
    let mut changed = None;
    let mut matched = None;
    let conditions = wait.url.is_some() || wait.changed.is_some() || wait.matching.is_some();
    let (reason, polls) = polled(wait.for_, conditions, poll, timer, || {
        // Each condition in turn, the first that holds ending the check before the next is asked: the files first,
        // since a probe may take its whole timeout.
        if let Some(watched) = &mut watched {
            changed = watched.changed(verbs)?;
            if changed.is_some() {
                return Ok(Some(Reason::Changed));
            }
        }
        if let Some(search) = &searching {
            matched = matched_line(verbs, search)?;
            if matched.is_some() {
                return Ok(Some(Reason::Matched));
            }
        }
        if wait.url.is_none() {
            return Ok(None);
        }
        let left = wait
            .for_
            .saturating_sub(timer.elapsed().saturating_sub(begun));
        Ok(match url.probed(left.clamp(poll.first, PROBE_LONGEST))? {
            Probed::Answered => Some(Reason::Answered),
            // A replay's recorded no-answer cannot change: with nothing else to watch, the deadline is its answer.
            Probed::Settled if !watches_files => Some(Reason::Deadline),
            Probed::NotYet | Probed::Settled => None,
        })
    })?;
    Ok(Answered {
        answer: WaitAnswer {
            reason,
            started_at,
            waited_ms: millis(timer.elapsed().saturating_sub(begun)),
            for_ms: millis(wait.for_),
            polls,
            url: url.last,
            changed,
            matched,
        },
        effect: url.effect.unwrap_or_else(Effect::performed),
    })
}

/// A wait's URL and its last probe.
struct Probing<'a> {
    verbs: &'a Verbs,
    ask: Option<Ask>,
    mode: EffectsMode,
    last: Option<Probe>,
    effect: Option<Effect>,
    /// A replay answered a no-answer from the journal: every later probe would answer the same.
    settled: bool,
}

/// What one probe came to.
enum Probed {
    Answered,
    NotYet,
    Settled,
}

impl<'a> Probing<'a> {
    fn new(verbs: &'a Verbs, wait: &Wait) -> Self {
        let ask = wait.url.as_ref().map(|url| Ask {
            method: "GET".to_string(),
            url: url.clone(),
            headers: Vec::new(),
            body: None,
            secret: None,
            // What every probe's record is found by, whatever time each takes: a replay finds the record of this
            // URL's probe at any `--for`.
            timeout: PROBE_LONGEST,
            // As `http`'s: a record keeps the answer only when it fits, and a replay answers from that record.
            max_bytes: http::ANSWER_BYTES,
            no_redirects: false,
        });
        Probing {
            verbs,
            ask,
            // A wait asks about now: a fresh record of an earlier one would end it at once.
            mode: match wait.mode {
                EffectsMode::Replay => EffectsMode::Replay,
                _ => EffectsMode::Online,
            },
            last: None,
            effect: None,
            settled: false,
        }
    }

    /// One probe, sent with `timeout`: a server answered (any status), or not yet, or (on replay) a recorded
    /// no-answer that stays one.
    fn probed(&mut self, timeout: Duration) -> Result<Probed, OpError> {
        let Some(ask) = &self.ask else {
            return Ok(Probed::NotYet);
        };
        if self.settled {
            return Ok(Probed::Settled);
        }
        match http::send(self.verbs, ask, self.mode, timeout)? {
            Sent::Answered(answered) => {
                self.last = Some(Probe {
                    status: Some(answered.answer.status),
                    ok: answered.answer.ok,
                    outcome: answered.answer.outcome,
                });
                self.effect = Some(answered.effect);
                Ok(Probed::Answered)
            }
            // No server listens there yet; any other class is the refusal `http` gives it.
            Sent::NoAnswer { class, effect } if http::no_answer(class).not_yet => {
                self.last = Some(Probe {
                    status: None,
                    ok: false,
                    outcome: class,
                });
                self.settled = effect.from_journal;
                self.effect = Some(effect);
                Ok(if self.settled {
                    Probed::Settled
                } else {
                    Probed::NotYet
                })
            }
            Sent::NoAnswer { class, .. } => Err(http::unanswered(&ask.url, class)),
        }
    }
}

/// `--changed`'s glob, judged: the directory it walks and the pattern below it, or the one file it names.
struct Watched {
    /// Absolute: the project's root joined to the glob's directory.
    at: PathBuf,
    /// Anchored at `at`; none when the glob names one file.
    pattern: Option<String>,
    since: UnixMillis,
    /// The most entries a poll's walk lists.
    max: usize,
    /// What the first check read.
    first: Option<Reading>,
}

impl Watched {
    /// `changed` split at its first component that holds a glob character, read from the project's root wherever the
    /// wait is typed, its pattern read as `files list` reads a glob and its place held to the roots; a write counts
    /// from `--since`, else from `started_at`.
    fn judged(verbs: &Verbs, changed: &Changed, started_at: UnixMillis) -> Result<Self, OpError> {
        if changed.glob.is_empty() {
            return Err(invalid(
                "the glob to watch is empty: write one, such as **/*.spec",
            ));
        }
        let root = verbs.root().dir();
        let parts: Vec<&str> = changed.glob.split('/').collect();
        let globbed = parts
            .iter()
            .position(|part| part.contains(['*', '?', '[', '{']));
        let (at, pattern) = match globbed {
            None => (root.join(&changed.glob), None),
            Some(index) => {
                let dir = parts[..index].join("/");
                let at = match (index, dir.as_str()) {
                    (0, _) => root.to_path_buf(),
                    // `/x`: the root directory.
                    (_, "") => PathBuf::from("/"),
                    _ => root.join(dir),
                };
                // A leading `/` anchors it at the directory, as a `/`-holding pattern already is.
                (at, Some(format!("/{}", parts[index..].join("/"))))
            }
        };
        if let Some(pattern) = &pattern
            && let Some(why) = verbs.ports().fs.glob_fault(std::slice::from_ref(pattern))
        {
            return Err(invalid(why));
        }
        verbs.held_for("wait", &at, FsOp::Stat)?;
        // A directory it names is watched by the files in it: its own time moves when a file is made or removed.
        let pattern = match pattern {
            None if stat(verbs, &at)?
                .1
                .is_some_and(|stat| stat.kind == FileKind::Dir) =>
            {
                Some("/**".to_string())
            }
            pattern => pattern,
        };
        Ok(Watched {
            at,
            pattern,
            since: changed.since.unwrap_or(started_at),
            max: files::LIST_MAX,
            first: None,
        })
    }

    /// The files written, as an answer shows them; none when there is none. The first check counts a file modified at
    /// or after `since` and not after the check (a time in the future is no write: a watch cycle's one check would
    /// see it on every cycle); each later one, a file whose time differs from the first check's, or that appeared or
    /// went since.
    fn changed(&mut self, verbs: &Verbs) -> Result<Option<Vec<String>>, OpError> {
        let reading = self.read(verbs)?;
        let written: Vec<String> = match &self.first {
            None => {
                let now = UnixMillis::now_or_end().0;
                let fresh = |at: &u64| (self.since.0..=now).contains(at);
                reading
                    .iter()
                    .filter(|(_, modified)| modified.as_ref().is_some_and(fresh))
                    .map(|(path, _)| path.clone())
                    .collect()
            }
            Some(first) => {
                let moved = reading
                    .iter()
                    .filter(|(path, modified)| first.get(*path) != Some(*modified));
                let gone = first
                    .iter()
                    .filter(|(path, _)| !reading.contains_key(*path));
                let mut written: Vec<String> =
                    moved.chain(gone).map(|(path, _)| path.clone()).collect();
                written.sort();
                written
            }
        };
        self.first.get_or_insert(reading);
        Ok((!written.is_empty()).then_some(written))
    }

    /// Each file the glob names now, as an answer shows it, and its modification time; none when the place does not
    /// exist yet.
    fn read(&self, verbs: &Verbs) -> Result<Reading, OpError> {
        let Some(pattern) = &self.pattern else {
            let (located, stat) = stat(verbs, &self.at)?;
            return Ok(stat
                .filter(|stat| stat.kind == FileKind::File)
                .map(|stat| (verbs.shown(&located.at), stat.modified_ms))
                .into_iter()
                .collect());
        };
        let walked = files::walk(
            verbs,
            &self.at,
            Reach {
                max: self.max,
                ..Reach::files(std::slice::from_ref(pattern))
            },
        );
        let (located, listing) = match walked {
            Ok(walked) => walked,
            Err(error) if error.kind == OpErrorKind::FileNotFound => return Ok(Reading::new()),
            Err(error) => return Err(error),
        };
        // A write in what the walk did not read would go unseen, a false quiet.
        let unseen = if listing.truncated {
            Some(format!(
                "the walk of {} stopped at its bound ({} files listed, {} entries visited)",
                verbs.shown(&located.at),
                self.max,
                specforge_effects::fs::LIST_VISITS
            ))
        } else {
            listing
                .unread
                .first()
                .map(|rel| format!("{} could not be read", verbs.shown_in(&located.root, rel)))
        };
        if let Some(why) = unseen {
            return Err(OpError::new(
                OpErrorKind::PreconditionFailed,
                OpErrorKind::PreconditionFailed.as_str(),
                format!(
                    "{why}, so a write in what it did not read would go unseen: narrow the glob"
                ),
            ));
        }
        Ok(listing
            .entries
            .iter()
            .map(|entry| {
                (
                    verbs.shown_in(&located.root, &entry.path),
                    entry.modified_ms,
                )
            })
            .collect())
    }
}

/// The files a glob names, as an answer shows them, and their modification times.
type Reading = BTreeMap<String, Option<u64>>;

/// `--match --in` as `files search` reads it, judged before any time passes: the file held to the roots and the
/// pattern parsed.
fn searched(verbs: &Verbs, matching: &Matching) -> Result<Search, OpError> {
    verbs.held_for("wait", &matching.file, FsOp::Stat)?;
    one_file(verbs, &matching.file)?;
    let search = Search {
        max: 1,
        ..Search::new(matching.pattern.clone(), vec![matching.file.clone()])
    };
    files::checked_patterns(&search)?;
    Ok(search)
}

/// The first line `search` finds; none when none matches, or the file does not exist yet.
fn matched_line(verbs: &Verbs, search: &Search) -> Result<Option<Matched>, OpError> {
    for path in &search.paths {
        one_file(verbs, path)?;
    }
    let answer = files::search(verbs, search)?;
    Ok(answer.matches.into_iter().next().map(|found| Matched {
        path: found.path,
        line: found.line,
        text: found.text,
    }))
}

/// `path`'s stat through the Fs port, held to the roots; none when it does not exist yet.
fn stat(verbs: &Verbs, path: &Path) -> Result<(Located, Option<FsStat>), OpError> {
    let (located, request) = verbs.held(path, FsOp::Stat)?;
    let outcome = verbs.ports().fs.run(verbs.roots(), &request);
    match outcome.class {
        OutcomeClass::Ok => Ok((located, outcome.stat)),
        OutcomeClass::Missing => Ok((located, None)),
        class => Err(refused(class, &verbs.shown(&located.at))),
    }
}

/// Refused when `--in` names a directory, which `files search` would search whole: a wait reads one file.
fn one_file(verbs: &Verbs, path: &Path) -> Result<(), OpError> {
    match stat(verbs, path)? {
        (located, Some(stat)) if stat.kind == FileKind::Dir => Err(invalid(format!(
            "{} is a directory: a wait searches the lines of one file",
            verbs.shown(&located.at)
        ))),
        _ => Ok(()),
    }
}

/// An `invalid_input` refusal.
fn invalid(why: impl Into<String>) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        OpErrorKind::InvalidInput.as_str(),
        why,
    )
}

/// Refused unless `url` is an `http(s)` URL with a host that the user grant permits a `GET` of.
fn judge_url(verbs: &Verbs, url: &str) -> Result<(), OpError> {
    let scheme = scheme(url);
    if !matches!(scheme.as_str(), "http" | "https")
        || host_of(url).is_none_or(|host| host.is_empty())
    {
        return Err(invalid(format!(
            "{} is no URL to wait for: write http:// or https:// and a host",
            specforge_common::remedy::printable(url)
        )));
    }
    verbs.permit(&EffectRequest::Http(HttpRequest {
        method: "GET".to_string(),
        url: url.to_string(),
        ..HttpRequest::default()
    }))
}

/// The poll: with `conditions`, `check` at once, after each pause and once more at the deadline,
/// until it names a reason; with none, one pause of `for_`. The reason and how many checks ran.
fn polled(
    for_: Duration,
    conditions: bool,
    poll: Poll,
    timer: &dyn Timer,
    mut check: impl FnMut() -> Result<Option<Reason>, OpError>,
) -> Result<(Reason, u64), OpError> {
    let begun = timer.elapsed();
    let spent = || timer.elapsed().saturating_sub(begun);
    if !conditions {
        timer.pause(for_);
        return Ok((Reason::Elapsed, 0));
    }
    let mut polls = 0;
    let mut pause = poll.first;
    loop {
        polls += 1;
        if let Some(reason) = check()? {
            return Ok((reason, polls));
        }
        let left = for_.saturating_sub(spent());
        if left.is_zero() {
            return Ok((Reason::Deadline, polls));
        }
        timer.pause(pause.min(left));
        pause = (pause * 2).min(poll.max);
    }
}

fn millis(span: Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
