//! What one call may do through the `specforge:host` ports (ADR 0335 D4): a [`PortSession`], made per call from its
//! kind and what its caller lent ([`PortCall`]). It decides every request before any effect, and it is the one place
//! a guest's request meets the host: what only the host may set is taken out of it here, in every runtime.
//!
//! An answer holds what may reach a guest and nothing else (ADR 0288): an attached secret is the granted port's, on
//! the host side, and a program a guest runs starts from an empty environment plus what its capability declares, less
//! any variable a declared secret names.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specforge_common::{Diagnostic, codes};
use specforge_effects::journal::{Answer, Blob, LentRecord};
use specforge_effects::{
    Denial as Refusal, Denied, EffectJournal, EffectRequest, Effects, Grant, Granted, Outcome,
    OutcomeClass, Ports, Roots, SharedFs, SharedGit, UnixMillis,
};
use specforge_protocol_types::{Denial, Failure, HostError, NotParsed, Operation};
use specforge_source::{SourceReader, SyntaxCache, SyntaxWork};

use crate::sandbox::{ANSWER_SHARE_OF_MEMORY, Limits, PORT_BUDGET_MS};

mod fs;
mod git;
mod http;
mod process;
mod record;
mod syntax;

/// What an effectful call is lent: the effects its http and process requests are served through (the mode, the ports
/// a live mode holds, the journal), the local ports every mode performs, and its extension's grant.
#[derive(Clone)]
pub struct PortCall {
    pub effects: Arc<Effects>,
    pub fs: SharedFs,
    pub git: SharedGit,
    pub roots: Arc<Roots>,
    pub grant: Arc<Grant>,
    /// Every variable a declared secret names, of any extension: no program a guest runs inherits one.
    pub secrets: Vec<String>,
    /// The host's source reader a `syntax` request parses with: one per collect, so its queries are memoised and its
    /// cache shared (a detached cache when the caller lends none).
    pub syntax: Arc<SourceReader>,
    /// What the caller reports of this call's refused requests.
    denials: Arc<Mutex<Denials>>,
    /// What the caller reports of the source files its `syntax` requests could not parse or parsed broken, and the
    /// work they took.
    notes: Arc<Mutex<SyntaxNotes>>,
}

/// How many source files a W196 or an I211 names; past them it counts.
const NAMED_FILES: usize = 8;

/// The source files of one call's `syntax` requests that a note names: how many, and the first [`NAMED_FILES`].
#[derive(Default)]
struct Named {
    count: usize,
    first: Vec<String>,
}

impl Named {
    fn add(&mut self, path: &str, why: String) {
        self.count += 1;
        if self.first.len() < NAMED_FILES {
            self.first.push(format!("`{path}` ({why})"));
        }
    }

    /// `<count> <what>: <first>, …, and N more`.
    fn said(&self, what: &str) -> String {
        let mut said = format!("{} {what}: {}", self.count, self.first.join(", "));
        if self.count > self.first.len() {
            said.push_str(&format!(", and {} more", self.count - self.first.len()));
        }
        said
    }
}

/// One call's notes of its `syntax` requests.
#[derive(Default)]
struct SyntaxNotes {
    not_parsed: Named,
    broken: Named,
    work: SyntaxWork,
}

/// How many denials one call keeps: a guest in a loop may not fill the host's memory with its own requests.
const KEPT_DENIALS: usize = 32;
/// The most of a refused request's text a denial keeps, in bytes.
const DENIED_TEXT: usize = 256;

/// What one call was refused: the first [`KEPT_DENIALS`], and how many more there were.
#[derive(Default)]
struct Denials {
    kept: Vec<Diagnostic>,
    more: usize,
    /// What [`PortCall::deny_once`] already said.
    said: BTreeSet<String>,
}

/// `text` held to [`DENIED_TEXT`].
fn cut_denied(mut text: String) -> String {
    if text.len() > DENIED_TEXT {
        text.truncate(text.floor_char_boundary(DENIED_TEXT));
        text.push_str("...");
    }
    text
}

impl PortCall {
    /// `grant` over `ports`, its http and process requests served through `effects`: the run's mode (ADR 0337 D4).
    pub fn new(
        effects: Effects,
        ports: &Ports,
        roots: Roots,
        grant: Grant,
        secrets: Vec<String>,
    ) -> Self {
        PortCall {
            effects: Arc::new(effects),
            fs: Arc::clone(&ports.fs),
            git: Arc::clone(&ports.git),
            roots: Arc::new(roots),
            grant: Arc::new(grant),
            secrets,
            syntax: Arc::new(SourceReader::new(SyntaxCache::detached())),
            denials: Arc::default(),
            notes: Arc::default(),
        }
    }

    /// This call, its `syntax` requests parsed by `reader` (a project's cache, shared across a collect's calls).
    pub fn with_syntax(mut self, reader: Arc<SourceReader>) -> Self {
        self.syntax = reader;
        self
    }

    /// The work this call's `syntax` requests took so far, taken.
    pub fn take_syntax_work(&self) -> SyntaxWork {
        std::mem::take(&mut self.notes().work)
    }

    fn notes(&self) -> std::sync::MutexGuard<'_, SyntaxNotes> {
        self.notes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A file a `syntax` request could not parse (W196), parsed with `errors` (I211), and the work it took.
    fn noted(&self, not_parsed: &[(&str, NotParsed)], broken: &[(&str, u32)], work: SyntaxWork) {
        let mut notes = self.notes();
        for (path, reason) in not_parsed {
            notes.not_parsed.add(path, reason.name().to_string());
        }
        for (path, errors) in broken {
            let plural = if *errors == 1 { "" } else { "s" };
            notes.broken.add(path, format!("{errors} error{plural}"));
        }
        notes.work += work;
    }

    /// [`Self::new`], `online`, with a record in `journal`.
    pub fn online(
        ports: &Ports,
        journal: EffectJournal,
        roots: Roots,
        grant: Grant,
        secrets: Vec<String>,
    ) -> Self {
        let effects = Effects::online(ports.clone(), journal);
        Self::new(effects, ports, roots, grant, secrets)
    }

    fn denials(&self) -> std::sync::MutexGuard<'_, Denials> {
        self.denials
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Keeps `denial` for the caller to report, up to [`KEPT_DENIALS`] of them; past that it is only counted.
    pub fn deny(&self, denial: Diagnostic) {
        let mut denials = self.denials();
        if denials.kept.len() < KEPT_DENIALS {
            denials.kept.push(denial);
        } else {
            denials.more += 1;
        }
    }

    /// [`Self::deny`], unless this call already said `denial`.
    fn deny_once(&self, denial: Diagnostic) {
        if self.denials().said.insert(denial.message.clone()) {
            self.deny(denial);
        }
    }

    /// The denials kept so far, taken, the last one saying how many were not kept, then once each the source files
    /// its `syntax` requests did not parse (W196) and parsed with syntax errors (I211): what the caller reports once
    /// the call ended.
    pub fn take_denials(&self) -> Vec<Diagnostic> {
        let Denials { mut kept, more, .. } = std::mem::take(&mut *self.denials());
        let extension = self.grant.extension();
        if more > 0 {
            kept.push(Diagnostic::new(
                codes::E090,
                format!("extension '{extension}': {more} more requests were refused"),
            ));
        }
        let mut notes = self.notes();
        let not_parsed = std::mem::take(&mut notes.not_parsed);
        if not_parsed.count > 0 {
            kept.push(Diagnostic::new(
                codes::W196,
                format!(
                    "extension '{extension}': {}",
                    not_parsed.said("source files were not parsed")
                ),
            ));
        }
        let broken = std::mem::take(&mut notes.broken);
        if broken.count > 0 {
            kept.push(Diagnostic::new(
                codes::I211,
                format!(
                    "extension '{extension}': {}",
                    broken.said("source files parsed with syntax errors")
                ),
            ));
        }
        kept
    }
}

/// What one call may do, and what it has spent.
pub struct PortSession {
    extension: String,
    kind: Option<Operation>,
    lent: Option<PortCall>,
    /// Any byte payload of an answer is held to this.
    answer_cap: u64,
    memory_mb: u32,
    /// What its requests took so far: not the guest's time, and held to the port budget.
    in_ports: Duration,
}

/// What a port's failure names that no [`OutcomeClass`] does: nothing was performed for its mode, no record holds its
/// answer, git could not answer, the project is in no repository.
const NOT_PERFORMED: &str = "not_performed";
const NOT_IN_JOURNAL: &str = "not_in_journal";
const UNAVAILABLE: &str = "unavailable";
const NOT_A_REPOSITORY: &str = "not_a_repository";

fn failed(class: impl ToString, message: impl Into<String>) -> HostError {
    HostError::Failed(Failure {
        class: class.to_string(),
        message: message.into(),
    })
}

fn out_of_scope(why: impl Into<String>) -> HostError {
    HostError::Denied(Denial::Scope(why.into()))
}

/// `bytes` held to `cap`, and whether that cut them.
fn held(mut bytes: Vec<u8>, cap: u64) -> (Vec<u8>, bool) {
    let cut = bytes.len() as u64 > cap;
    bytes.truncate(usize::try_from(cap).unwrap_or(usize::MAX));
    (bytes, cut)
}

/// What a guest asks a cap to be (0: the most it may have), held to `most`.
fn asked_cap(asked: u64, most: u64) -> u64 {
    if asked == 0 { most } else { asked.min(most) }
}

fn millis(span: std::time::Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(u64::MAX)
}

impl PortSession {
    /// A call of a kind no port is for, or made outside any session: every port answers `denied(call-kind)`.
    pub fn pure() -> Self {
        PortSession {
            extension: String::new(),
            kind: None,
            lent: None,
            answer_cap: 0,
            memory_mb: 0,
            in_ports: Duration::ZERO,
        }
    }

    /// A call of `extension`, of `kind`, lent `lent` (none: no port is lent, so every request is `denied(scope)`),
    /// under `limits`.
    pub fn effectful(
        extension: &str,
        kind: Operation,
        lent: Option<PortCall>,
        limits: Limits,
    ) -> Self {
        PortSession {
            extension: extension.to_string(),
            kind: Some(kind),
            lent,
            answer_cap: u64::from(limits.memory_mb) * 1024 * 1024 / ANSWER_SHARE_OF_MEMORY,
            memory_mb: limits.memory_mb,
            in_ports: Duration::ZERO,
        }
    }

    /// How long this call has spent in its ports: what the guest's deadline does not count (ADR 0037 D3).
    pub fn time_in_ports(&self) -> Duration {
        self.in_ports
    }

    /// One port request, its time counted as the port's. A call that spent [`PORT_BUDGET_MS`] in its ports is
    /// answered `failed(deadline)`, as a guest cannot trap for time it does not count.
    pub fn timed<T>(
        &mut self,
        request: impl FnOnce(&mut Self) -> Result<T, HostError>,
    ) -> Result<T, HostError> {
        if self.budget_left() == 0 {
            return Err(failed(
                OutcomeClass::Deadline,
                format!("the call spent its {PORT_BUDGET_MS} ms budget in host ports"),
            ));
        }
        let started = Instant::now();
        let answer = request(self);
        self.in_ports += started.elapsed();
        answer
    }

    /// What is left of the call's budget for its ports, in milliseconds.
    fn budget_left(&self) -> u64 {
        PORT_BUDGET_MS.saturating_sub(millis(self.in_ports))
    }

    /// What the call was lent, when its kind may use a port at all.
    fn lent(&self) -> Result<(Operation, &PortCall), HostError> {
        let Some(kind) = self.kind.filter(|kind| kind.is_effectful()) else {
            return Err(HostError::Denied(Denial::CallKind(
                "only a command, an MCP tool or resource and a collector may call a host port"
                    .to_string(),
            )));
        };
        match &self.lent {
            Some(call) => Ok((kind, call)),
            None => Err(out_of_scope(format!(
                "no port is lent to this call of '{}'",
                self.extension
            ))),
        }
    }

    /// `refusal` as a guest is answered it (ADR 0335 D2.7), with what the caller reports of it kept: an E090 per request
    /// the grant refuses, and W170 once for what a person can approve, whose text holds nothing of the request.
    fn refused(call: &PortCall, refusal: Refusal) -> HostError {
        let message = format!("`{}`: {}", refusal.request, refusal.detail);
        let why = refusal.why;
        match why {
            Denied::CallKind | Denied::Undeclared | Denied::Ceiling => call.deny(
                Refusal {
                    request: cut_denied(refusal.request),
                    detail: cut_denied(refusal.detail),
                    ..refusal
                }
                .diagnostic(),
            ),
            Denied::NotApproved | Denied::Root => call.deny_once(Diagnostic::new(
                codes::W170,
                format!(
                    "extension '{}', capability '{}': {}",
                    refusal.extension,
                    refusal.capability.unwrap_or_default(),
                    refusal.detail
                ),
            )),
        }
        match why {
            Denied::CallKind => HostError::Denied(Denial::CallKind(message)),
            _ => HostError::Denied(Denial::Scope(message)),
        }
    }

    /// The capability that permits `request`, or why the grant refuses it.
    fn granted<'c>(
        kind: Operation,
        call: &'c PortCall,
        request: &EffectRequest,
    ) -> Result<&'c Granted, HostError> {
        call.grant
            .permitted(kind, request)
            .map_err(|refusal| Self::refused(call, refusal))
    }
}

/// Whose a request's record is: `call`'s extension, through `granted`, with the secret it attaches (ADR 0335 D4).
fn lent_record<'c>(call: &'c PortCall, granted: &'c Granted) -> LentRecord<'c> {
    LentRecord {
        extension: call.grant.extension(),
        capability: &granted.declared.name,
        secret_env: granted.secret.as_ref().map(|secret| secret.env.as_str()),
    }
}

/// An answer of `body` as the journal keeps it: its class, the body's digest and `payload` ([`record`]).
fn answered(class: OutcomeClass, body: &[u8], payload: Vec<u8>) -> Answer {
    Answer {
        outcome: Outcome {
            class,
            digest: Blob::of(body).sha256,
        },
        payload: Some(payload),
    }
}

fn now() -> UnixMillis {
    UnixMillis::now_or_end()
}

fn not_performed(request: &EffectRequest) -> HostError {
    failed(
        NOT_PERFORMED,
        format!(
            "`{}` is not performed in this call's effects mode",
            specforge_effects::grant::one_line(request)
        ),
    )
}

fn not_in_journal(request: &EffectRequest) -> HostError {
    failed(
        NOT_IN_JOURNAL,
        format!(
            "`{}` is not in the effects journal",
            specforge_effects::grant::one_line(request)
        ),
    )
}

#[cfg(test)]
mod tests {
    use specforge_effects::process::ProcessClass;
    use specforge_effects::testing::{self, FakeHttp, FakeProcess, MemoryConsent};
    use specforge_effects::{EffectsCeiling, HttpRequest, Provenance};
    use specforge_extension_sdk::prelude::{Capability, ContributionsBuilder, ExtensionMeta};
    use specforge_protocol_types::ProcessRequest;
    use std::path::Path;

    use super::*;

    #[test]
    fn a_spent_port_budget_fails_the_request_before_it_runs() {
        let mut session = PortSession::pure();
        session.in_ports = Duration::from_millis(PORT_BUDGET_MS);
        let mut ran = false;
        let answer: Result<(), HostError> = session.timed(|_| {
            ran = true;
            Ok(())
        });
        assert!(
            matches!(&answer, Err(HostError::Failed(failure)) if failure.class == "deadline"),
            "{answer:?}"
        );
        assert!(!ran);
    }

    /// A session of `journal`'s, with most of its port budget spent when `spent`.
    fn session(effects: Effects, ports: &Ports, root: &Path, spent: bool) -> PortSession {
        let mut c = ContributionsBuilder::new(ExtensionMeta::new("@t/a", "1.0.0"));
        c.capability(Capability::http("api").hosts(&["example.test"]).why("api"));
        c.capability(Capability::process("run").argv(&["cargo"]).why("run"));
        let roots = Roots::new(root, root);
        let grant = Grant::decide(
            &c.declaration(),
            &Provenance::third_party(b"x"),
            &EffectsCeiling::default(),
            &roots,
            &MemoryConsent::approving_everything(),
        );
        let call = PortCall::new(effects, ports, roots, grant, Vec::new());
        let mut session =
            PortSession::effectful("@t/a", Operation::Collect, Some(call), Limits::CEILING);
        if spent {
            session.in_ports = Duration::from_millis(PORT_BUDGET_MS - 100);
        }
        session
    }

    /// A clamp to what the budget left shortens what runs: the same request finds its record however much was spent.
    #[test]
    fn spending_the_budget_first_does_not_change_a_requests_record() {
        let dir = tempfile::tempdir().unwrap();
        let ports = Ports {
            http: Arc::new(FakeHttp::new().route("https://example.test/a", 200, "answer")),
            process: Arc::new(FakeProcess::new().answering(
                "cargo",
                ProcessClass::Exited(0),
                "built",
            )),
            ..testing::ports()
        };
        let journal = || EffectJournal::at(dir.path());
        let http = || HttpRequest {
            method: "GET".to_string(),
            url: "https://example.test/a".to_string(),
            ..HttpRequest::default()
        };
        let process = || ProcessRequest {
            argv: vec!["cargo".to_string()],
            ..ProcessRequest::default()
        };
        let online = || Effects::online(ports.clone(), journal());
        let mut live = session(online(), &ports, dir.path(), false);
        let live_http = live.http(http()).unwrap();
        let live_process = live.process_run(process()).unwrap();

        let replay = Effects::replay(journal());
        let mut spent = session(replay, &ports, dir.path(), true);
        assert_eq!(spent.http(http()).unwrap(), live_http);
        assert_eq!(spent.process_run(process()).unwrap(), live_process);
    }

    /// What runs is held to the budget all the same.
    #[test]
    fn what_runs_is_held_to_what_the_budget_left() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(FakeHttp::new().route("https://example.test/a", 200, "answer"));
        let ports = Ports {
            http: fake.clone(),
            ..testing::ports()
        };
        let online = Effects::online(ports.clone(), EffectJournal::at(dir.path()));
        let mut spent = session(online, &ports, dir.path(), true);
        spent
            .http(HttpRequest {
                method: "GET".to_string(),
                url: "https://example.test/a".to_string(),
                ..HttpRequest::default()
            })
            .unwrap();
        assert!(fake.requests()[0].timeout_ms <= 100);
    }

    #[test]
    fn a_requests_time_is_the_ports_and_shortens_the_next_deadline() {
        let mut session = PortSession::pure();
        let answer: Result<(), HostError> = session.timed(|_| {
            std::thread::sleep(Duration::from_millis(5));
            Ok(())
        });
        assert!(answer.is_ok());
        assert!(session.time_in_ports() >= Duration::from_millis(5));

        session.in_ports = Duration::from_millis(PORT_BUDGET_MS - 100);
        assert_eq!(session.budget_left(), 100);
    }
}
