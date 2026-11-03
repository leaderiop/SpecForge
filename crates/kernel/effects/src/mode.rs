//! The effects mode (ADR 0337 D4): whether a run performs an http or process effect, answers it from the effects
//! journal, or replays the journal. [`Effects`] is the one entry point; one opened `cached` or `replay` holds no
//! ports, so it cannot perform an effect, and an fs effect is local: the caller performs it in every mode.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::journal::{self, Answer, EffectJournal, Kept, Lookup, RecordDigest, RecordedRequest};
use crate::{EffectRequest, Ports, UnixMillis};

/// The four modes, as `--effects` and MCP `validate`'s `effects` name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum EffectsMode {
    /// Performed when granted; the answer is journaled.
    Online,
    /// A fresh record; else performed when approved; else not performed.
    Auto,
    /// A fresh record; else not performed.
    Cached,
    /// Any record, expired or not; else a miss.
    Replay,
}

impl EffectsMode {
    pub const ALL: [EffectsMode; 4] = [
        EffectsMode::Online,
        EffectsMode::Auto,
        EffectsMode::Cached,
        EffectsMode::Replay,
    ];

    pub const NAMES: [&'static str; 4] = ["online", "auto", "cached", "replay"];

    pub fn name(self) -> &'static str {
        match self {
            EffectsMode::Online => "online",
            EffectsMode::Auto => "auto",
            EffectsMode::Cached => "cached",
            EffectsMode::Replay => "replay",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// Whether the mode performs an effect through the ports (`online`, `auto`).
    pub fn is_live(self) -> bool {
        matches!(self, EffectsMode::Online | EffectsMode::Auto)
    }
}

/// Whether this machine approved an effect (plan 02's grants answer it; until then the caller says).
pub type Approved = Arc<dyn Fn(&EffectRequest) -> bool + Send + Sync>;

/// A run's effects: its mode, the ports a live mode performs through, the journal, and what the run consulted.
pub struct Effects {
    mode: EffectsMode,
    /// Held for `online` and `auto` only: the constructors keep it so.
    ports: Option<Ports>,
    approved: Approved,
    journal: EffectJournal,
    log: Mutex<Log>,
}

impl fmt::Debug for Effects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Effects")
            .field("mode", &self.mode)
            .field("journal", &self.journal)
            .finish_non_exhaustive()
    }
}

/// What [`Effects::serve`] answered a request: the answer, never the request, is what leaves toward a guest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Served {
    /// The answer (class, digest and payload), and whether it came from the journal.
    Outcome { answer: Answer, from_journal: bool },
    /// Not performed in this mode; the check that asked is left unchecked (I206).
    NotPerformed(Why),
    /// `replay` found no record (the run is refused `not_in_journal`).
    Miss,
    /// An fs effect: a local read the caller performs in every mode, never journaled.
    Local,
}

/// Why an effect was not performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// `auto`, and this machine did not approve it.
    NotApproved,
    /// `cached`, and the journal holds no fresh record.
    NoFreshRecord,
}

/// A request `replay` found no record of: what the refusal lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missed {
    pub digest: RecordDigest,
    pub request: RecordedRequest,
}

/// `GET https://… (sha256:…)`.
impl fmt::Display for Missed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.request, self.digest)
    }
}

/// What one run consulted: each record file looked up, with when it expires (an input of the checks, ADR 0337 D5),
/// how many effects were not performed per scope (I206), and what `replay` missed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Log {
    /// Each record file consulted, with when its answer expires (none: the clock changes nothing, as for a record
    /// already expired, missing, or never served fresh).
    pub consulted: BTreeMap<PathBuf, Option<UnixMillis>>,
    /// Effects not performed, counted by the scope that asked (`links`, `registries`, …).
    pub not_run: BTreeMap<String, usize>,
    /// The requests `replay` found no record of.
    pub misses: Vec<Missed>,
}

impl Effects {
    fn new(
        mode: EffectsMode,
        ports: Option<Ports>,
        approved: Approved,
        journal: EffectJournal,
    ) -> Self {
        Effects {
            mode,
            ports,
            approved,
            journal,
            log: Mutex::default(),
        }
    }

    /// `cached` over `journal`: no port.
    pub fn cached(journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Cached, None, Arc::new(|_| false), journal)
    }

    /// `replay` over `journal`: no port.
    pub fn replay(journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Replay, None, Arc::new(|_| false), journal)
    }

    /// `online` through `ports`: every granted effect is performed.
    pub fn online(ports: Ports, journal: EffectJournal) -> Self {
        Self::new(
            EffectsMode::Online,
            Some(ports),
            Arc::new(|_| true),
            journal,
        )
    }

    /// `auto` through `ports`: a fresh record, else an effect `approved` says this machine approved.
    pub fn auto(ports: Ports, approved: Approved, journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Auto, Some(ports), approved, journal)
    }

    /// `mode` over `journal`, through `ports` and `approved` for a live mode.
    pub fn of(mode: EffectsMode, ports: Ports, approved: Approved, journal: EffectJournal) -> Self {
        match mode {
            EffectsMode::Online => Effects::online(ports, journal),
            EffectsMode::Auto => Effects::auto(ports, approved, journal),
            EffectsMode::Cached => Effects::cached(journal),
            EffectsMode::Replay => Effects::replay(journal),
        }
    }

    pub fn mode(&self) -> EffectsMode {
        self.mode
    }

    /// The ports a live mode performs through; none for `cached` and `replay`.
    pub fn ports(&self) -> Option<&Ports> {
        self.ports.as_ref()
    }

    pub fn journal(&self) -> &EffectJournal {
        &self.journal
    }

    /// What this run consulted so far.
    pub fn log(&self) -> Log {
        self.log_lock().clone()
    }

    /// What this run consulted, leaving the log empty for the next run (a session runs the checks again over one
    /// environment, and each run's inputs are its own).
    pub fn take_log(&self) -> Log {
        std::mem::take(&mut *self.log_lock())
    }

    fn log_lock(&self) -> std::sync::MutexGuard<'_, Log> {
        self.log
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Answer `request`, asked for `scope`, at `now` as the mode says; `perform` runs it through the ports when the
    /// mode performs it. A performed answer is journaled as the TTL table says ([`journal::retention`]).
    pub fn serve(
        &self,
        scope: &str,
        request: &EffectRequest,
        now: UnixMillis,
        perform: impl FnOnce(&Ports) -> Answer,
    ) -> Served {
        let Some(shown) = RecordedRequest::of(request) else {
            return Served::Local;
        };
        let digest = RecordDigest::of(request);
        let found = self.journal.lookup(&digest, now);
        let mut log = self.log_lock();
        if let Some(path) = self.journal.path(&digest) {
            // Only a fresh record's answer changes with the clock: an expired or missing one changes when written.
            let expires = match &found {
                Lookup::Fresh(kept) => journal::expires(&kept.record),
                Lookup::Expired(_) | Lookup::Missing => None,
            };
            log.consulted.insert(path, expires);
        }
        let answered = |kept: Kept| Served::Outcome {
            answer: kept.answer(),
            from_journal: true,
        };
        let mut not_run = |why| {
            *log.not_run.entry(scope.to_string()).or_default() += 1;
            Served::NotPerformed(why)
        };
        let ports = match (self.mode, found) {
            (EffectsMode::Replay, Lookup::Fresh(kept) | Lookup::Expired(kept)) => {
                return answered(kept);
            }
            (EffectsMode::Replay, Lookup::Missing) => {
                log.misses.push(Missed {
                    digest,
                    request: shown,
                });
                return Served::Miss;
            }
            (EffectsMode::Cached | EffectsMode::Auto, Lookup::Fresh(kept)) => {
                return answered(kept);
            }
            (EffectsMode::Cached, _) => return not_run(Why::NoFreshRecord),
            (EffectsMode::Auto, _) if !(self.approved)(request) => {
                return not_run(Why::NotApproved);
            }
            (EffectsMode::Online | EffectsMode::Auto, _) => {
                self.ports.as_ref().expect("a live mode holds its ports")
            }
        };
        drop(log);
        let answer = perform(ports);
        // A journal that cannot be written costs the cache, not the answer.
        if let Ok(Some((path, record))) = self.journal.keep(request, &answer, now) {
            self.log_lock()
                .consulted
                .insert(path, journal::expires(&record));
        }
        Served::Outcome {
            answer,
            from_journal: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::journal::HTTP_SUCCESS_TTL;
    use crate::testing::ports;
    use crate::{FsOp, FsRequest, HttpRequest, Outcome, OutcomeClass};

    fn http(url: &str) -> EffectRequest {
        EffectRequest::Http(HttpRequest {
            method: "GET".into(),
            url: url.into(),
            ..HttpRequest::default()
        })
    }

    fn answer(class: OutcomeClass) -> Answer {
        Answer {
            outcome: Outcome {
                class,
                digest: String::new(),
            },
            payload: None,
        }
    }

    fn at(ms: u64) -> UnixMillis {
        UnixMillis(ms)
    }

    /// A journal under a fresh directory, holding `request` answered `ok` at `at`.
    fn journal_with(request: &EffectRequest, when: u64) -> (tempfile::TempDir, EffectJournal) {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        journal
            .keep(request, &answer(OutcomeClass::Ok), at(when))
            .unwrap();
        (dir, journal)
    }

    /// Serve `request` at `now`, counting how often the ports were asked; they answer `class`.
    fn serve_as(
        effects: &Effects,
        request: &EffectRequest,
        now: u64,
        performed: &Cell<u32>,
        class: OutcomeClass,
    ) -> Served {
        effects.serve("links", request, at(now), |_| {
            performed.set(performed.get() + 1);
            answer(class)
        })
    }

    fn serve(
        effects: &Effects,
        request: &EffectRequest,
        now: u64,
        performed: &Cell<u32>,
    ) -> Served {
        serve_as(effects, request, now, performed, OutcomeClass::Status(500))
    }

    #[test]
    fn cached_answers_a_fresh_record_and_performs_nothing() {
        let request = http("https://example.org/");
        let (_dir, journal) = journal_with(&request, 0);
        let effects = Effects::cached(journal);
        assert!(effects.ports().is_none(), "cached holds no port");
        let performed = Cell::new(0);
        assert_eq!(
            serve(&effects, &request, 1, &performed),
            Served::Outcome {
                answer: answer(OutcomeClass::Ok),
                from_journal: true,
            }
        );
        let other = http("https://example.org/other");
        assert_eq!(
            serve(&effects, &other, 1, &performed),
            Served::NotPerformed(Why::NoFreshRecord)
        );
        let expired = HTTP_SUCCESS_TTL.as_millis() as u64;
        assert_eq!(
            serve(&effects, &request, expired, &performed),
            Served::NotPerformed(Why::NoFreshRecord)
        );
        assert_eq!(performed.get(), 0);
        let log = effects.log();
        assert_eq!(log.not_run.get("links"), Some(&2));
        assert_eq!(log.consulted.len(), 2, "each record file looked up");
        // Served expired last: the clock changes its answer no more, so it carries no expiry (ADR 0337 D5).
        assert!(log.consulted.values().all(Option::is_none));
        let fresh = Effects::cached(effects.journal().clone());
        serve(&fresh, &request, 1, &performed);
        assert_eq!(
            fresh.take_log().consulted.into_values().collect::<Vec<_>>(),
            vec![Some(at(expired))],
            "a fresh record expires with its TTL"
        );
        assert!(
            fresh.log().consulted.is_empty(),
            "taken: the next run starts empty"
        );
    }

    #[test]
    fn auto_performs_only_an_approved_effect_and_journals_it() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let approved: Approved =
            Arc::new(|r| matches!(r, EffectRequest::Http(h) if h.url.contains("approved")));
        let effects = Effects::auto(ports(), approved, journal.clone());
        let performed = Cell::new(0);
        let refused = http("https://example.org/");
        assert_eq!(
            serve(&effects, &refused, 1, &performed),
            Served::NotPerformed(Why::NotApproved)
        );
        let asked = http("https://example.org/approved");
        assert_eq!(
            serve(&effects, &asked, 1, &performed),
            Served::Outcome {
                answer: answer(OutcomeClass::Status(500)),
                from_journal: false,
            }
        );
        assert_eq!(performed.get(), 1);
        assert!(matches!(
            journal.lookup(&RecordDigest::of(&asked), at(2)),
            Lookup::Fresh(_)
        ));
        // Its fresh record answers the next run without the port.
        assert!(matches!(
            serve(&effects, &asked, 2, &performed),
            Served::Outcome {
                from_journal: true,
                ..
            }
        ));
        assert_eq!(performed.get(), 1);
    }

    #[test]
    fn a_denied_answer_is_not_served_after_a_grant() {
        let dir = tempfile::tempdir().unwrap();
        let journal = EffectJournal::at(dir.path());
        let effects = Effects::auto(ports(), Arc::new(|_| true), journal.clone());
        let request = http("https://example.org/");
        let performed = Cell::new(0);
        // The port refuses it: not granted yet. No record is kept.
        assert!(matches!(
            serve_as(&effects, &request, 1, &performed, OutcomeClass::Denied),
            Served::Outcome {
                from_journal: false,
                ..
            }
        ));
        assert_eq!(
            journal.lookup(&RecordDigest::of(&request), at(2)),
            Lookup::Missing
        );
        // Granted since: the next run performs it at once.
        assert_eq!(
            serve_as(&effects, &request, 2, &performed, OutcomeClass::Ok),
            Served::Outcome {
                answer: answer(OutcomeClass::Ok),
                from_journal: false,
            }
        );
        assert_eq!(performed.get(), 2);
    }

    #[test]
    fn online_performs_past_a_fresh_record() {
        let request = http("https://example.org/");
        let (_dir, journal) = journal_with(&request, 0);
        let effects = Effects::online(ports(), journal.clone());
        let performed = Cell::new(0);
        assert!(matches!(
            serve(&effects, &request, 1, &performed),
            Served::Outcome {
                from_journal: false,
                ..
            }
        ));
        assert_eq!(performed.get(), 1);
        let Lookup::Fresh(kept) = journal.lookup(&RecordDigest::of(&request), at(2)) else {
            panic!("the new answer replaced the record");
        };
        assert_eq!(kept.record.outcome.class, OutcomeClass::Status(500));
    }

    #[test]
    fn replay_serves_an_expired_record_and_logs_a_miss() {
        let request = http("https://example.org/");
        let (_dir, journal) = journal_with(&request, 0);
        let effects = Effects::replay(journal);
        let performed = Cell::new(0);
        let late = HTTP_SUCCESS_TTL.as_millis() as u64 * 10;
        assert!(matches!(
            serve(&effects, &request, late, &performed),
            Served::Outcome {
                from_journal: true,
                ..
            }
        ));
        let other = http("https://example.org/other");
        assert_eq!(serve(&effects, &other, late, &performed), Served::Miss);
        let misses = effects.log().misses;
        assert_eq!(misses.len(), 1);
        assert_eq!(
            misses[0].to_string(),
            format!(
                "GET https://example.org/other ({})",
                RecordDigest::of(&other)
            )
        );
        assert_eq!(performed.get(), 0);
    }

    #[test]
    fn fs_and_git_are_performed_in_every_mode() {
        // An fs effect is local: every mode hands it back to the caller, unjournaled. A git question is no
        // `EffectRequest`: the session's Git port answers it in every mode.
        let dir = tempfile::tempdir().unwrap();
        let read = EffectRequest::Fs(FsRequest {
            root: "project".into(),
            path: "a.txt".into(),
            op: FsOp::Read { max_bytes: 1 },
        });
        let journal = EffectJournal::at(dir.path());
        let performed = Cell::new(0);
        for mode in EffectsMode::ALL {
            let effects = Effects::of(mode, ports(), Arc::new(|_| false), journal.clone());
            assert_eq!(effects.mode(), mode);
            assert_eq!(effects.ports().is_some(), mode.is_live());
            assert_eq!(
                serve(&effects, &read, 0, &performed),
                Served::Local,
                "{mode:?}"
            );
            assert_eq!(effects.log(), Log::default());
        }
        assert_eq!(performed.get(), 0);
        assert!(!dir.path().join(journal::JOURNAL_DIR).exists());
        assert_eq!(EffectsMode::parse("replay"), Some(EffectsMode::Replay));
        assert_eq!(
            EffectsMode::NAMES.map(|n| EffectsMode::parse(n).unwrap().name()),
            EffectsMode::NAMES
        );
    }
}
