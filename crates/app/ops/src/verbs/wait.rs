//! `specforge wait` (ADR 0342): a wait for a URL to answer, a file to be written or a line to match, under the user
//! grant, polled through the Http and Fs ports until a condition holds or its time is up. Every condition is judged before any time passes; then it is checked at once,
//! after a pause that doubles to a bound, and once more at the deadline. With no condition it is a delay.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Serialize;
use specforge_effects::fs::FileKind;
use specforge_effects::http::{host_of, scheme};
use specforge_effects::{EffectRequest, EffectsMode, FsOp, OutcomeClass, UnixMillis};
use specforge_protocol_types::HttpRequest;

use super::files::{self, Reach, Search, SearchMode, Stdin};
use super::http::{self, Ask, UNREACHABLE};
use super::{Answered, Effect, Verbs, refused};
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

/// The shortest and longest time a URL probe may take, the wait's own time held between them: at least a connection's
/// time, at most a short request's.
const PROBE_TIME: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(10));

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

/// Why a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub outcome: String,
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

/// A wait's clock: how long since it began, and a pause. A test passes its own, and waits no real time.
pub trait Clock {
    fn elapsed(&self) -> Duration;
    fn pause(&self, span: Duration);
}

/// The machine's monotonic clock, from when it was made.
#[derive(Debug, Clone, Copy)]
pub struct RealTime(Instant);

impl RealTime {
    pub fn start() -> Self {
        RealTime(Instant::now())
    }
}

impl Clock for RealTime {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn pause(&self, span: Duration) {
        std::thread::sleep(span);
    }
}

/// `wait`, judged first, then polled by `poll` on `clock`.
pub fn wait(
    verbs: &Verbs,
    wait: &Wait,
    poll: Poll,
    clock: &dyn Clock,
) -> Result<Answered<WaitAnswer>, OpError> {
    let begun = clock.elapsed();
    let started_at = UnixMillis::now_or_end();
    if let Some(url) = &wait.url {
        judge_url(verbs, url)?;
    }
    let watched = wait
        .changed
        .as_ref()
        .map(|changed| Watched::judged(verbs, changed, started_at))
        .transpose()?;
    if let Some(matching) = &wait.matching {
        let (located, _) = verbs.held(&matching.file, FsOp::Stat)?;
        verbs.beyond_machine_roots(&located, "wait")?;
    }
    let mut url = Probing::new(verbs, wait);
    let mut changed = None;
    let mut matched = None;
    let conditions = wait.url.is_some() || wait.changed.is_some() || wait.matching.is_some();
    let mut first = true;
    let (reason, polls) = polled(wait.for_, conditions, poll, clock, || {
        // The first check runs every condition, so each one's refusal comes before any time passes; the files first,
        // since a probe may take its whole timeout.
        let every = std::mem::take(&mut first);
        let mut held = None;
        if let Some(watched) = &watched {
            changed = watched.changed(verbs)?;
            held = held.or(changed.as_ref().map(|_| Reason::Changed));
        }
        if let Some(matching) = &wait.matching
            && (every || held.is_none())
        {
            matched = matched_line(verbs, matching)?;
            held = held.or(matched.as_ref().map(|_| Reason::Matched));
        }
        if wait.url.is_some() && (every || held.is_none()) && url.answered()? {
            held = held.or(Some(Reason::Answered));
        }
        Ok(held)
    })?;
    Ok(Answered {
        answer: WaitAnswer {
            reason,
            started_at,
            waited_ms: millis(clock.elapsed().saturating_sub(begun)),
            for_ms: millis(wait.for_),
            polls,
            url: url.last,
            changed: changed.filter(|_| reason == Reason::Changed),
            matched: matched.filter(|_| reason == Reason::Matched),
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
}

impl<'a> Probing<'a> {
    fn new(verbs: &'a Verbs, wait: &Wait) -> Self {
        let ask = wait.url.as_ref().map(|url| Ask {
            method: "GET".to_string(),
            url: url.clone(),
            headers: Vec::new(),
            body: None,
            secret: None,
            // The same for every probe: the request, so its record's digest, holds it, and a replay of this wait
            // finds it.
            timeout: wait.for_.clamp(PROBE_TIME.0, PROBE_TIME.1),
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
        }
    }

    /// One probe: whether a server answered, any status.
    fn answered(&mut self) -> Result<bool, OpError> {
        let Some(ask) = &self.ask else {
            return Ok(false);
        };
        match http::request(self.verbs, ask, self.mode) {
            Ok(answered) => {
                self.last = Some(Probe {
                    status: Some(answered.answer.status),
                    ok: answered.answer.ok,
                    outcome: answered.answer.outcome,
                });
                self.effect = Some(answered.effect);
                Ok(true)
            }
            // No server answered yet; a denied redirect is a refusal.
            Err(error)
                if error.code == UNREACHABLE && error.kind != OpErrorKind::PermissionDenied =>
            {
                self.last = Some(Probe {
                    status: None,
                    ok: false,
                    outcome: error
                        .data
                        .as_ref()
                        .and_then(|data| data["outcome"].as_str())
                        .unwrap_or(UNREACHABLE)
                        .to_string(),
                });
                Ok(false)
            }
            Err(error) => Err(error),
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
}

impl Watched {
    /// `changed` split at its first component that holds a glob character, read from the project's root wherever the
    /// wait is typed, its pattern read as `files list` reads a glob and its place held to the roots; a write counts
    /// from `--since`, else from `started_at`.
    fn judged(verbs: &Verbs, changed: &Changed, started_at: UnixMillis) -> Result<Self, OpError> {
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
            return Err(OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                why,
            ));
        }
        let (located, _) = verbs.held(&at, FsOp::Stat)?;
        verbs.beyond_machine_roots(&located, "wait")?;
        Ok(Watched {
            at,
            pattern,
            since: changed.since.unwrap_or(started_at),
        })
    }

    /// The files written at or after `since`, as an answer shows them; none when there is none, or the place does not
    /// exist yet.
    fn changed(&self, verbs: &Verbs) -> Result<Option<Vec<String>>, OpError> {
        let fresh = |modified: Option<u64>| modified.is_some_and(|at| at >= self.since.0);
        let Some(pattern) = &self.pattern else {
            let (located, request) = verbs.held(&self.at, FsOp::Stat)?;
            let outcome = verbs.ports().fs.run(verbs.roots(), &request);
            return match outcome.class {
                OutcomeClass::Ok => Ok(outcome
                    .stat
                    .filter(|stat| fresh(stat.modified_ms))
                    .map(|_| vec![verbs.shown(&located.at)])),
                OutcomeClass::Missing => Ok(None),
                class => Err(refused(class, &verbs.shown(&located.at))),
            };
        };
        let walked = files::walk(
            verbs,
            &self.at,
            Reach {
                globs: std::slice::from_ref(pattern),
                kind: Some(FileKind::File),
                depth: None,
                ignore: true,
                hidden: false,
                max: files::LIST_MAX,
            },
        );
        let (located, listing) = match walked {
            Ok(walked) => walked,
            Err(error) if error.kind == OpErrorKind::FileNotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let written: Vec<String> = listing
            .entries
            .iter()
            .filter(|entry| fresh(entry.modified_ms))
            .map(|entry| verbs.shown_in(&located.root, &entry.path))
            .collect();
        Ok((!written.is_empty()).then_some(written))
    }
}

/// The first line of `--in` that `--match` matches, searched as `files search` searches; none when none does, or the
/// file does not exist yet.
fn matched_line(verbs: &Verbs, matching: &Matching) -> Result<Option<Matched>, OpError> {
    let answer = files::search(
        verbs,
        &Search {
            pattern: matching.pattern.clone(),
            paths: vec![matching.file.clone()],
            globs: Vec::new(),
            ignore: true,
            hidden: false,
            ignore_case: false,
            fixed: false,
            invert: false,
            multiline: false,
            before: 0,
            after: 0,
            and_not: Vec::new(),
            mode: SearchMode::Lines,
            max: 1,
            stdin: Stdin::default(),
        },
    )?;
    Ok(answer.matches.into_iter().next().map(|found| Matched {
        path: found.path,
        line: found.line,
        text: found.text,
    }))
}

/// Refused unless `url` is an `http(s)` URL with a host that the user grant permits a `GET` of.
fn judge_url(verbs: &Verbs, url: &str) -> Result<(), OpError> {
    let scheme = scheme(url);
    if !matches!(scheme.as_str(), "http" | "https")
        || host_of(url).is_none_or(|host| host.is_empty())
    {
        return Err(OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            format!(
                "{} is no URL to wait for: write http:// or https:// and a host",
                specforge_common::remedy::printable(url)
            ),
        ));
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
    clock: &dyn Clock,
    mut check: impl FnMut() -> Result<Option<Reason>, OpError>,
) -> Result<(Reason, u64), OpError> {
    let begun = clock.elapsed();
    let spent = || clock.elapsed().saturating_sub(begun);
    if !conditions {
        clock.pause(for_);
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
        clock.pause(pause.min(left));
        pause = (pause * 2).min(poll.max);
    }
}

fn millis(span: Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    /// A clock that moves only when paused.
    #[derive(Default)]
    struct Fake {
        now: Cell<Duration>,
        pauses: RefCell<Vec<Duration>>,
    }

    impl Clock for Fake {
        fn elapsed(&self) -> Duration {
            self.now.get()
        }

        fn pause(&self, span: Duration) {
            self.now.set(self.now.get() + span);
            self.pauses.borrow_mut().push(span);
        }
    }

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn with_no_condition_it_pauses_its_whole_time_and_checks_nothing() {
        let clock = Fake::default();
        let ended = polled(MS(3000), false, Poll::BACK_OFF, &clock, || {
            panic!("no condition is checked")
        });
        assert_eq!(ended.unwrap(), (Reason::Elapsed, 0));
        assert_eq!(*clock.pauses.borrow(), [MS(3000)]);
    }

    #[test]
    fn it_checks_at_once_then_backs_off_to_the_bound_and_once_at_the_deadline() {
        let clock = Fake::default();
        let mut at = Vec::new();
        let ended = polled(MS(3000), true, Poll::BACK_OFF, &clock, || {
            at.push(millis(clock.elapsed()));
            Ok(None)
        });
        assert_eq!(ended.unwrap(), (Reason::Deadline, 7));
        assert_eq!(at, [0, 100, 300, 700, 1500, 2500, 3000]);
    }

    #[test]
    fn it_ends_at_the_first_check_that_holds() {
        let clock = Fake::default();
        let mut checks = 0;
        let ended = polled(MS(30_000), true, Poll::BACK_OFF, &clock, || {
            checks += 1;
            Ok((checks == 3).then_some(Reason::Answered))
        });
        assert_eq!(ended.unwrap(), (Reason::Answered, 3));
        assert_eq!(clock.elapsed(), MS(300));
    }

    #[test]
    fn a_wait_of_zero_checks_once() {
        let clock = Fake::default();
        let ended = polled(Duration::ZERO, true, Poll::BACK_OFF, &clock, || Ok(None));
        assert_eq!(ended.unwrap(), (Reason::Deadline, 1));
        assert!(clock.pauses.borrow().is_empty());
    }

    #[test]
    fn a_retry_after_is_read_in_every_form_and_held_to_a_day() {
        let now = std::time::UNIX_EPOCH + Duration::from_secs(784_111_777); // Sun, 06 Nov 1994 08:49:37 GMT
        for (value, waits) in [
            ("120", MS(120_000)),
            ("0", Duration::ZERO),
            ("1.5", MS(1500)),
            ("0.0001", MS(1)),
            ("86401", LONGEST),
            ("99999999999999999999", LONGEST),
            ("Sun, 06 Nov 1994 08:50:37 GMT", MS(60_000)),
            ("Sunday, 06-Nov-94 08:50:37 GMT", MS(60_000)),
            ("Sun Nov  6 08:50:37 1994", MS(60_000)),
            ("Sun, 06 Nov 1994 08:48:37 GMT", Duration::ZERO),
            ("Mon, 07 Nov 1994 08:49:38 GMT", LONGEST),
        ] {
            assert_eq!(retry_after(value, now), Ok(waits), "{value}");
        }
        for value in ["", "-5", "1e3", "inf", "1.2.3", "soon", "5m"] {
            assert!(retry_after(value, now).is_err(), "{value}");
        }
    }

    #[test]
    fn a_refusal_ends_the_wait_at_once() {
        let clock = Fake::default();
        let ended = polled(MS(30_000), true, Poll::BACK_OFF, &clock, || {
            Err(OpError::new(OpErrorKind::PermissionDenied, "x", "refused"))
        });
        assert_eq!(ended.unwrap_err().code, "x");
        assert_eq!(clock.elapsed(), Duration::ZERO);
    }
}
