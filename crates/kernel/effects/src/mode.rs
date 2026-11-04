//! The effects mode (ADR 0337 D4): whether a run performs an http or process effect, answers it from the effects
//! journal, or replays the journal. [`Effects`] is the one entry point; one opened `cached` or `replay` holds no
//! ports, so it cannot perform an effect, and an fs effect is local: the caller performs it in every mode.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use specforge_protocol_types::Operation;

use crate::grant::Grant;
use crate::journal::{
    self, Answer, EffectJournal, Kept, LentRecord, Lookup, RecordDigest, RecordedRequest,
};
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

/// Who asks for an effect: an extension, in a call of one kind, within the capabilities its asker names.
#[derive(Debug, Clone, Copy)]
pub struct Asker<'a> {
    pub extension: &'a str,
    pub op: Operation,
    pub scopes: &'a [String],
    /// The capability that permits the request posts queries ([`Grant::queries`]): its POST reads, and a fresh record
    /// answers it again as one answers a GET.
    pub queries: bool,
}

/// A run's effects: its mode, the ports a live mode performs through, the journal, and what the run consulted.
pub struct Effects {
    mode: EffectsMode,
    /// Held for `online` and `auto` only: the constructors keep it so.
    ports: Option<Ports>,
    /// What each extension may make a live mode do, by name (`specforge_project::effects` decides them).
    grants: BTreeMap<String, Arc<Grant>>,
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
    Outcome {
        answer: Answer,
        from_journal: bool,
        /// When it was observed: its record's time, else now.
        observed_at: UnixMillis,
    },
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
        grants: Vec<Grant>,
        journal: EffectJournal,
    ) -> Self {
        Effects {
            mode,
            ports,
            grants: grants
                .into_iter()
                .map(|grant| (grant.extension().to_string(), Arc::new(grant)))
                .collect(),
            journal,
            log: Mutex::default(),
        }
    }

    /// `cached` over `journal`: no port.
    pub fn cached(journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Cached, None, Vec::new(), journal)
    }

    /// `replay` over `journal`: no port.
    pub fn replay(journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Replay, None, Vec::new(), journal)
    }

    /// `online` through `ports`, for a caller who judges each request itself ([`Self::serve`]); it holds no grant, so
    /// [`Self::serve_for`] performs nothing.
    pub fn online(ports: Ports, journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Online, Some(ports), Vec::new(), journal)
    }

    /// `auto` through `ports`: a request is performed when the asker's own grant in `grants` permits it.
    pub fn auto(ports: Ports, grants: Vec<Grant>, journal: EffectJournal) -> Self {
        Self::new(EffectsMode::Auto, Some(ports), grants, journal)
    }

    /// `mode` over `journal`; a live mode performs through `ports`, each request judged by the asker's grant in
    /// `grants`. The other modes perform nothing, and hold neither.
    pub fn of(mode: EffectsMode, ports: Ports, grants: Vec<Grant>, journal: EffectJournal) -> Self {
        match mode {
            EffectsMode::Online | EffectsMode::Auto => {
                Self::new(mode, Some(ports), grants, journal)
            }
            EffectsMode::Cached => Effects::cached(journal),
            EffectsMode::Replay => Effects::replay(journal),
        }
    }

    /// `extension`'s grant in a live mode; none when it declares no capability, and in `cached` and `replay`.
    pub fn grant(&self, extension: &str) -> Option<&Arc<Grant>> {
        self.grants.get(extension)
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

    /// Count `n` effects of `scope` as not performed (I206): what a caller gave up before asking, as over its budget.
    pub fn count_not_run(&self, scope: &str, n: usize) {
        *self
            .log_lock()
            .not_run
            .entry(scope.to_string())
            .or_default() += n;
    }

    fn log_lock(&self) -> std::sync::MutexGuard<'_, Log> {
        self.log
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Answer `request` for `asker`, judged by the asker's own grant within its scopes: another extension's approval
    /// approves nothing here, and neither does one for another kind of call. `perform` gets the ports under that grant,
    /// so each redirect is granted again and a secret goes only where the grant attaches it. In a live mode a
    /// request the grant refuses is not performed and no record answers it. In every mode the record is the asker's
    /// alone ([`RecordDigest::asked_by`]), so a mode that holds no grant still serves no one another's answer.
    pub fn serve_for(
        &self,
        asker: &Asker<'_>,
        scope: &str,
        request: &EffectRequest,
        now: UnixMillis,
        perform: impl FnOnce(&Ports) -> Answer,
    ) -> Served {
        let digest = RecordDigest::asked_by(asker.extension, request);
        let again = answers_again(request, asker.queries);
        if !self.mode.is_live() || RecordedRequest::of(request).is_none() {
            return self.served(
                scope,
                request,
                &digest,
                Asking::Unjudged,
                again,
                now,
                perform,
            );
        }
        let grant = self
            .grants
            .get(asker.extension)
            .map(|grant| Arc::new(grant.only(asker.scopes)))
            .filter(|grant| grant.permits(asker.op, request).is_ok());
        let Some(grant) = grant else {
            self.count_not_run(scope, 1);
            return Served::NotPerformed(Why::NotApproved);
        };
        self.served(
            scope,
            request,
            &digest,
            Asking::Judged,
            again,
            now,
            |ports| {
                perform(&Ports {
                    process: grant.process(asker.op, Arc::clone(&ports.process)),
                    http: grant.http(asker.op, Arc::clone(&ports.http)),
                    fs: grant.fs(asker.op, Arc::clone(&ports.fs)),
                    git: Arc::clone(&ports.git),
                })
            },
        )
    }

    /// Answer `request`, asked for `scope`, at `now` as the mode says, for a caller who judged it: `perform` runs it
    /// through the raw ports, so it holds its grant itself. In `auto` nothing unjudged is performed
    /// ([`Self::serve_for`]). A performed answer is journaled as the TTL table says ([`journal::retention`]).
    pub fn serve(
        &self,
        scope: &str,
        request: &EffectRequest,
        now: UnixMillis,
        perform: impl FnOnce(&Ports) -> Answer,
    ) -> Served {
        let digest = RecordDigest::of(request);
        let again = answers_again(request, false);
        self.served(
            scope,
            request,
            &digest,
            Asking::Unjudged,
            again,
            now,
            perform,
        )
    }

    /// [`Self::serve`] for a guest's request (ADR 0335 D4): its record is `lent`'s own, so a guest is never answered
    /// another extension's, or a capability's it did not ask through, or a host-run check's.
    pub fn serve_lent(
        &self,
        lent: LentRecord<'_>,
        request: &EffectRequest,
        now: UnixMillis,
        perform: impl FnOnce(&Ports) -> Answer,
    ) -> Served {
        let digest = RecordDigest::lent(request, lent);
        self.served(
            lent.capability,
            request,
            &digest,
            Asking::Lent,
            answers_again(request, false),
            now,
            perform,
        )
    }

    /// The one decision: `request`, found by `digest`, for one who is `asking`; a fresh record answers it only
    /// when it answers `again` ([`answers_again`]).
    #[allow(clippy::too_many_arguments, reason = "one decision's every input")]
    fn served(
        &self,
        scope: &str,
        request: &EffectRequest,
        digest: &RecordDigest,
        asking: Asking,
        again: bool,
        now: UnixMillis,
        perform: impl FnOnce(&Ports) -> Answer,
    ) -> Served {
        let Some(shown) = RecordedRequest::of(request) else {
            return Served::Local;
        };
        let judged = asking != Asking::Unjudged;
        let found = self.journal.lookup(digest, now);
        let mut log = self.log_lock();
        if let Some(path) = self.journal.path(digest) {
            // Only a fresh record's answer changes with the clock: an expired or missing one changes when written.
            let expires = match &found {
                Lookup::Fresh(kept) => journal::expires(&kept.record),
                Lookup::Expired(_) | Lookup::Missing => None,
            };
            log.consulted.insert(path, expires);
        }
        let answered = |kept: Kept| Served::Outcome {
            observed_at: kept.record.observed_at,
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
                    digest: digest.clone(),
                    request: shown,
                });
                return Served::Miss;
            }
            (EffectsMode::Cached | EffectsMode::Auto, Lookup::Fresh(kept)) if again => {
                return answered(kept);
            }
            (EffectsMode::Cached, _) => return not_run(Why::NoFreshRecord),
            (EffectsMode::Auto, _) if !judged => return not_run(Why::NotApproved),
            (EffectsMode::Online | EffectsMode::Auto, _) => {
                self.ports.as_ref().expect("a live mode holds its ports")
            }
        };
        drop(log);
        let answer = perform(ports);
        let kept = if asking == Asking::Lent {
            self.journal.keep_lent(digest, request, &answer, now)
        } else {
            self.journal.keep_as(digest, request, &answer, now)
        };
        // A journal that cannot be written costs the cache, not the answer.
        if let Ok(Some((path, record))) = kept {
            self.log_lock()
                .consulted
                .insert(path, journal::expires(&record));
        }
        Served::Outcome {
            answer,
            from_journal: false,
            observed_at: now,
        }
    }
}

/// Who a request is served for, which decides what a record may answer and what a live mode may perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asking {
    /// A caller who holds its grant itself: `auto` performs nothing for it.
    Unjudged,
    /// One whose grant [`Effects::serve_for`] judged.
    Judged,
    /// A guest whose grant permitted the request ([`Effects::serve_lent`]).
    Lent,
}

/// Whether a fresh record may stand in for `request`, for every asker (ADR 0335 D4): only when it asks for nothing to
/// happen. That is a `GET` or a `HEAD`, a name's addresses, a handshake, and a POST whose capability `queries`
/// ([`Grant::queries`]). Any other request asked for something to happen, and a day-old answer says it did. A process
/// has no fresh record at all.
fn answers_again(request: &EffectRequest, queries: bool) -> bool {
    match request {
        EffectRequest::Http(http) => {
            ["GET", "HEAD"]
                .iter()
                .any(|method| http.method.eq_ignore_ascii_case(method))
                || (queries && http.method.eq_ignore_ascii_case("POST"))
        }
        EffectRequest::Resolve(_) | EffectRequest::Tls(_) => true,
        EffectRequest::Process(_) | EffectRequest::Fs(_) => false,
    }
}

#[cfg(test)]
mod tests;
