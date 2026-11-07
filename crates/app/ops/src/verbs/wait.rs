//! `specforge wait` (ADR 0342): a wait for a URL to answer, under the user grant, polled through the ports until a
//! condition holds or its time is up. Every condition is judged before any time passes; then it is checked at once,
//! after a pause that doubles to a bound, and once more at the deadline. With no condition it is a delay.

use std::time::{Duration, Instant};

use serde::Serialize;
use specforge_effects::http::{host_of, scheme};
use specforge_effects::{EffectRequest, EffectsMode, UnixMillis};
use specforge_protocol_types::HttpRequest;

use super::http::{self, Ask, UNREACHABLE};
use super::{Answered, Effect, Verbs};
use crate::{OpError, OpErrorKind};

/// The longest wait, as long as `run`'s longest deadline.
pub const LONGEST: Duration = specforge_effects::process::USER_CEILING;
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
    /// `online`, `auto` and `cached` probe live; `replay` answers each probe from the journal.
    pub mode: EffectsMode,
}

/// Why a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The URL answered.
    Answered,
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
    // A wait asks about now: a fresh record of an earlier one would end it at once.
    let mode = match wait.mode {
        EffectsMode::Replay => EffectsMode::Replay,
        _ => EffectsMode::Online,
    };
    let mut probe = None;
    let mut effect = None;
    let conditions = wait.url.is_some();
    // The same for every probe: the request, so its record's digest, holds it, and a replay of this wait finds it.
    let probe_time = wait.for_.clamp(PROBE_TIME.0, PROBE_TIME.1);
    let (reason, polls) = polled(wait.for_, conditions, poll, clock, || {
        let Some(url) = &wait.url else {
            return Ok(None);
        };
        let ask = Ask {
            method: "GET".to_string(),
            url: url.clone(),
            headers: Vec::new(),
            body: None,
            secret: None,
            timeout: probe_time,
            // As `http`'s: a record keeps the answer only when it fits, and a replay answers from that record.
            max_bytes: http::ANSWER_BYTES,
            no_redirects: false,
        };
        match http::request(verbs, &ask, mode) {
            Ok(answered) => {
                probe = Some(Probe {
                    status: Some(answered.answer.status),
                    ok: answered.answer.ok,
                    outcome: answered.answer.outcome,
                });
                effect = Some(answered.effect);
                Ok(Some(Reason::Answered))
            }
            // No server answered yet; a denied redirect is a refusal.
            Err(error)
                if error.code == UNREACHABLE && error.kind != OpErrorKind::PermissionDenied =>
            {
                probe = Some(Probe {
                    status: None,
                    ok: false,
                    outcome: error
                        .data
                        .as_ref()
                        .and_then(|data| data["outcome"].as_str())
                        .unwrap_or(UNREACHABLE)
                        .to_string(),
                });
                Ok(None)
            }
            Err(error) => Err(error),
        }
    })?;
    Ok(Answered {
        answer: WaitAnswer {
            reason,
            started_at,
            waited_ms: millis(clock.elapsed().saturating_sub(begun)),
            for_ms: millis(wait.for_),
            polls,
            url: probe,
        },
        effect: effect.unwrap_or_else(Effect::performed),
    })
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
    fn a_refusal_ends_the_wait_at_once() {
        let clock = Fake::default();
        let ended = polled(MS(30_000), true, Poll::BACK_OFF, &clock, || {
            Err(OpError::new(OpErrorKind::PermissionDenied, "x", "refused"))
        });
        assert_eq!(ended.unwrap_err().code, "x");
        assert_eq!(clock.elapsed(), Duration::ZERO);
    }
}
