//! What a checker's probes ask, performed by the host (ADR 0336): each path contained, each request gated on the
//! checker's scopes, one request per digest, and the network's through the run's effects. A probe not performed leaves
//! its value not judged, so nothing goes red offline.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use specforge_common::remedy::printable;
use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_effects::contained::{Contained, Refused, Resolved};
use specforge_effects::grant::{Denied, Uses, one_line};
use specforge_effects::http::{self, DEFAULT_TIMEOUT};
use specforge_effects::{
    EffectRequest, Effects, EffectsMode, FsOp, FsRequest, Grant, HttpRequest, NoConsent,
    OutcomeClass, ResolveRequest, Roots, TlsRequest, UnixMillis,
};
use specforge_protocol_types::{
    Header, Operation, OutcomeData, PrefixProblem, Probe, ProbeOutcome, Root, RootedPath,
};
use specforge_registry::{CheckCall, Disk, DiskLookup};

use super::budget::{self, ValueBudget};
use super::collect::Value;
use super::local::{self, WALK_CAP, git_of, stat_of};
use super::network::{HEAD_ROOM, Job, Performed, Spending, perform_all};
use crate::Environment;

/// The kind of call a probe is asked under.
const OP: Operation = Operation::CheckValues;
/// The most of a file `fs.read_prefix` and `fs.read` answer, and of a body `http.get` does.
const PREFIX_CAP: u32 = 64 * 1024;
const READ_CAP: u32 = 1024 * 1024;
const BODY_CAP: u32 = 64 * 1024;
/// The response headers a checker may read.
const KEPT_HEADERS: [&str; 5] = [
    "content-length",
    "content-type",
    "etag",
    "last-modified",
    "location",
];

/// One checker's call on one value, with the probes its plan asks.
pub(super) struct Asked<'a> {
    pub value: &'a Value<'a>,
    pub call: &'a CheckCall,
    pub probes: &'a [Probe],
}

/// What became of one [`Asked`]: every probe's answer in order, unless one was refused or not performed.
#[derive(Debug, Default)]
pub(super) struct Reading {
    pub outcomes: Vec<ProbeOutcome>,
    /// What the host itself reports on the value (E090, E094, E095, W174, W175); the probe is not performed.
    pub refused: Vec<Diagnostic>,
    /// The class of a refusal that fails the value.
    pub refused_class: Option<OutcomeClass>,
    pub not_run: bool,
}

impl Reading {
    /// Whether the checker may judge it: only what was performed reaches it.
    pub fn answered(&self) -> bool {
        !self.not_run && self.refused.is_empty()
    }
}

/// A probe's place in its [`Reading`].
enum Slot {
    Done(ProbeOutcome),
    Job(usize),
    /// Refused or not performed: the reading says so.
    Stopped,
}

/// Perform what `asked` asks through `effects` and `disk`: a reading for each, and W170 for what this machine has not
/// approved of the scopes these values use (O11).
pub(super) fn dispatch(
    env: &Environment,
    effects: &Effects,
    disk: &dyn Disk,
    asked: &[Asked<'_>],
) -> (Vec<Reading>, Vec<Diagnostic>) {
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(env.root(), env.spec_root()));
    let lookup = DiskLookup(disk);
    let budget = env.value_budget();
    let deadline = Instant::now() + budget.time;
    let mut probing = Probing {
        env,
        effects,
        disk,
        budget,
        deadline,
        spent: 0,
        contained: Contained::new(&roots, &lookup),
        now: UnixMillis::now_or_end(),
        gates: gates(env, &roots, asked),
        local: BTreeMap::new(),
        jobs: Vec::new(),
        by_digest: HashMap::new(),
        uses: BTreeMap::new(),
    };
    let mut readings: Vec<Reading> = Vec::with_capacity(asked.len());
    let mut slots: Vec<Vec<Slot>> = Vec::with_capacity(asked.len());
    for one in asked {
        let mut reading = Reading::default();
        if one.probes.len() > budget.probes_per_value {
            effects.count_not_run(budget::OVER, 1);
            reading.not_run = true;
            slots.push(Vec::new());
        } else {
            slots.push(
                one.probes
                    .iter()
                    .map(|probe| probing.prepare(one, probe, &mut reading))
                    .collect(),
            );
        }
        readings.push(reading);
    }
    let spending = Spending {
        deadline,
        bytes: budget.bytes,
        spent: probing.spent,
    };
    let answers = perform_all(effects, probing.now, &probing.jobs, spending);
    let mut counted = vec![false; probing.jobs.len()];
    for ((one, reading), slots) in asked.iter().zip(&mut readings).zip(slots) {
        for slot in slots {
            let outcome = match slot {
                Slot::Done(outcome) => Some(outcome),
                Slot::Job(job) => {
                    let first = !std::mem::replace(&mut counted[job], true);
                    probing.answered(one, reading, &answers[job], &probing.jobs[job], first)
                }
                Slot::Stopped => None,
            };
            reading.outcomes.extend(outcome);
        }
    }
    let approvals = probing
        .uses
        .iter()
        .filter_map(|(extension, uses)| effects.grant(extension)?.unapproved_summary(uses))
        .collect();
    (readings, approvals)
}

/// Each asking extension's grant as nothing approved it: what it declares, its call kind and the project's ceiling
/// decide E090 the same in every mode, whatever this machine approved.
fn gates(env: &Environment, roots: &Roots, asked: &[Asked<'_>]) -> BTreeMap<String, Grant> {
    let mut gates = BTreeMap::new();
    for one in asked {
        let extension = &one.call.extension;
        if gates.contains_key(extension) {
            continue;
        }
        let Some(declaration) = env
            .registries()
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == extension)
        else {
            continue;
        };
        gates.insert(
            extension.clone(),
            Grant::decide(
                declaration,
                &crate::effects::provenance_of(env, extension),
                &env.effects_allowed().ceiling,
                roots,
                &NoConsent,
            ),
        );
    }
    gates
}

struct Probing<'a> {
    env: &'a Environment,
    effects: &'a Effects,
    disk: &'a dyn Disk,
    budget: ValueBudget,
    deadline: Instant,
    /// What the local probes read so far (the budget's `bytes`).
    spent: u64,
    contained: Contained<'a>,
    now: UnixMillis,
    gates: BTreeMap<String, Grant>,
    /// What the disk and git answered, per probe: one read however many values name a path.
    local: BTreeMap<Probe, Option<(OutcomeClass, OutcomeData)>>,
    jobs: Vec<Job<'a>>,
    /// The job of an extension's request: another extension is judged, and asks, for itself.
    by_digest: HashMap<(&'a str, String), usize>,
    /// What these values use of each extension's capabilities and this machine has not approved.
    uses: BTreeMap<String, Uses>,
}

impl<'a> Probing<'a> {
    /// Whether the budget still reads `probe` from the disk or git; one already read is free. When not, `one` is not
    /// judged and counts toward I206.
    fn local_budget(&self, one_reading: &mut Reading, probe: &Probe) -> bool {
        let spare = self.local.contains_key(probe)
            || (self.local.len() < self.budget.local_probes
                && self.spent < self.budget.bytes
                && Instant::now() < self.deadline);
        if !spare {
            self.effects.count_not_run(budget::OVER, 1);
            one_reading.not_run = true;
        }
        spare
    }

    /// Remember what `probe` read, and what it cost.
    fn read_local(&mut self, probe: &Probe, answer: &Option<(OutcomeClass, OutcomeData)>) {
        if let Some((_, data)) = answer {
            self.spent += budget::cost(data);
        }
        self.local.insert(probe.clone(), answer.clone());
    }

    fn prepare(&mut self, one: &Asked<'a>, probe: &'a Probe, reading: &mut Reading) -> Slot {
        let scopes = self
            .env
            .registries()
            .checks()
            .checker(&one.call.extension, &one.call.checker)
            .map_or(&[][..], |checker| checker.scopes.as_slice());
        match probe {
            Probe::FsStat { root, path } => {
                self.fs(one, probe, root, path, FsOp::Stat, scopes, reading)
            }
            Probe::FsReadPrefix { root, path, bytes } => {
                let max_bytes = u64::from((*bytes).min(PREFIX_CAP));
                self.fs(
                    one,
                    probe,
                    root,
                    path,
                    FsOp::Read { max_bytes },
                    scopes,
                    reading,
                )
            }
            Probe::FsRead { root, path, bytes } => {
                let max_bytes = u64::from((*bytes).min(READ_CAP));
                self.fs(
                    one,
                    probe,
                    root,
                    path,
                    FsOp::Read { max_bytes },
                    scopes,
                    reading,
                )
            }
            Probe::FsWalk { root, path, glob } => {
                let op = FsOp::Walk {
                    glob: Some(glob.clone()),
                    max_entries: WALK_CAP as u32,
                };
                self.fs(one, probe, root, path, op, scopes, reading)
            }
            Probe::GitObject { .. } | Probe::GitRef { .. } | Probe::GitTracked { .. } => {
                self.git(one, probe, scopes, reading)
            }
            Probe::HttpHead { .. }
            | Probe::HttpGet { .. }
            | Probe::HttpTls { .. }
            | Probe::DnsResolve { .. } => self.network(one, probe, scopes, reading),
        }
    }

    /// E094, E095, W174 or W175 for `written` under `default`, reported by the host on the value; else where it is.
    fn contain(
        &self,
        one: &Asked<'_>,
        default: &Root,
        written: &str,
        reading: &mut Reading,
    ) -> Option<Result<Resolved, ()>> {
        let value = one.value;
        let shown = printable(written);
        let at = |code, message: String, suggestion: String, span| {
            let mut diagnostic = Diagnostic::new(code, message).with_suggestion(suggestion);
            diagnostic.span = span;
            diagnostic.data = Some(Box::new(specforge_common::DiagnosticData::Subject {
                entity: value.entity.to_string(),
            }));
            diagnostic
        };
        let id = value.entity;
        let leaves = |why: &str| {
            at(
                codes::E094,
                format!("path '{shown}' in entity '{id}' leaves its root: {why}"),
                "name a file inside the root".to_string(),
                value.span.clone(),
            )
        };
        let refused = match self.contained.resolve(written, default.clone()) {
            Err(Refused::Empty) => return Some(Err(())),
            Err(Refused::UnknownRoot(name)) => {
                reading.refused_class = Some(OutcomeClass::Outside);
                at(
                    codes::E095,
                    format!(
                        "path '{shown}' in entity '{id}' names the root '<{name}>', which is not declared"
                    ),
                    "write `<spec>/…` or `<project>/…`, or declare the root in `roots`".to_string(),
                    root_span(value),
                )
            }
            Err(Refused::Absolute) => {
                reading.refused_class = Some(OutcomeClass::Outside);
                leaves("it is absolute")
            }
            Err(Refused::Climbs) => {
                reading.refused_class = Some(OutcomeClass::Outside);
                leaves("its `..` climbs above its root")
            }
            Err(Refused::LinksOut { .. }) => {
                reading.refused_class = Some(OutcomeClass::Outside);
                leaves("a link leads out of its root")
            }
            Ok(resolved) if resolved.backslashes => at(
                codes::W175,
                format!("path '{shown}' in entity '{id}' uses backslashes"),
                format!("write '{}'", shown.replace('\\', "/")),
                value.span.clone(),
            ),
            Ok(Resolved {
                case: Some(disk),
                exists: true,
                ..
            }) => {
                let disk = printable(&disk);
                at(
                    codes::W174,
                    format!(
                        "path '{shown}' in entity '{id}' differs in case from the file on disk, '{disk}'"
                    ),
                    format!("write '{disk}'"),
                    value.span.clone(),
                )
            }
            Ok(resolved) => return Some(Ok(resolved)),
        };
        // A warning leaves the value unread, so not judged; an escape fails it.
        if reading.refused_class.is_none() {
            self.effects.count_not_run(&one.call.checker, 1);
        }
        reading.refused.push(refused);
        None
    }

    /// Whether the checker's scopes reach `request`: E090 when they do not, in every mode. A root this machine has not
    /// approved is not performed, and asks for its approval.
    fn gated_fs(
        &mut self,
        one: &Asked<'_>,
        request: &EffectRequest,
        scopes: &[String],
        reading: &mut Reading,
    ) -> bool {
        let extension = one.call.extension.as_str();
        let Some(gate) = self.gates.get(extension) else {
            return false;
        };
        match gate.only(scopes).permits(OP, request) {
            Ok(()) => true,
            Err(denial) if denial.why == Denied::Root => {
                let live = self
                    .effects
                    .grant(extension)
                    .map(|grant| grant.only(scopes).permits(OP, request));
                match live {
                    Some(Ok(())) => true,
                    _ => {
                        if let Some(name) = denial.scope {
                            self.uses
                                .entry(extension.to_string())
                                .or_default()
                                .insert((OP, name));
                        }
                        self.effects.count_not_run(&one.call.checker, 1);
                        reading.not_run = true;
                        false
                    }
                }
            }
            Err(denial) => {
                refuse(reading, one.value, denial.diagnostic());
                false
            }
        }
    }

    /// A file probe: contained, gated, performed through the check's disk in every mode but `replay`.
    #[expect(clippy::too_many_arguments, reason = "one private step of `prepare`")]
    fn fs(
        &mut self,
        one: &Asked<'_>,
        probe: &'a Probe,
        root: &Root,
        written: &str,
        op: FsOp,
        scopes: &[String],
        reading: &mut Reading,
    ) -> Slot {
        let resolved = match self.contain(one, root, written, reading) {
            None => return Slot::Stopped,
            Some(resolved) => resolved,
        };
        let (rooted, rel) = match &resolved {
            Ok(resolved) => (resolved.root.as_str().to_string(), resolved.rel.clone()),
            Err(()) => (root.to_string(), String::new()),
        };
        let request = EffectRequest::Fs(FsRequest {
            root: rooted,
            path: rel,
            op: op.clone(),
        });
        if !self.gated_fs(one, &request, scopes, reading) {
            return Slot::Stopped;
        }
        if self.effects.mode() == EffectsMode::Replay {
            self.effects.count_not_run(&one.call.checker, 1);
            reading.not_run = true;
            return Slot::Stopped;
        }
        if !self.local_budget(reading, probe) {
            return Slot::Stopped;
        }
        let answer = match self.local.get(probe) {
            Some(answer) => answer.clone(),
            None => {
                let answer = Some(match &resolved {
                    Ok(resolved) if resolved.withheld => (OutcomeClass::Denied, OutcomeData::None),
                    Ok(resolved) => local::read_disk(self.disk, &self.contained, resolved, &op),
                    Err(()) => (OutcomeClass::Missing, stat_of(None, 0, None)),
                });
                self.read_local(probe, &answer);
                answer
            }
        };
        let (class, data) = answer.expect("a file probe always answers");
        Slot::Done(self.outcome(probe, class, data))
    }

    fn outcome(&self, probe: &Probe, class: OutcomeClass, data: OutcomeData) -> ProbeOutcome {
        ProbeOutcome {
            probe: probe.clone(),
            class,
            data,
            observed_at: self.now.0,
            from_cache: false,
        }
    }

    /// A git probe: gated on a git capability in the checker's scopes, asked of the Git port in every mode but
    /// `replay`, with no journal: its answer is local.
    fn git(
        &mut self,
        one: &Asked<'_>,
        probe: &'a Probe,
        scopes: &[String],
        reading: &mut Reading,
    ) -> Slot {
        let tracked = match probe {
            Probe::GitTracked { root, path } => match self.contain(one, root, path, reading) {
                None => return Slot::Stopped,
                Some(Ok(resolved)) => Some(resolved),
                Some(Err(())) => {
                    return Slot::Done(self.outcome(
                        probe,
                        OutcomeClass::Missing,
                        git_of(false, None),
                    ));
                }
            },
            _ => None,
        };
        let question = one_probe(probe);
        let Some(gate) = self.gates.get(&one.call.extension) else {
            return Slot::Stopped;
        };
        if let Err(denial) = gate.only(scopes).reads_git(OP, &question) {
            refuse(reading, one.value, denial.diagnostic());
            return Slot::Stopped;
        }
        if self.effects.mode() != EffectsMode::Replay && !self.local_budget(reading, probe) {
            return Slot::Stopped;
        }
        let answer = match (self.effects.mode(), self.local.get(probe)) {
            (EffectsMode::Replay, _) => None,
            (_, Some(answer)) => answer.clone(),
            (_, None) => {
                let answer = local::ask_git(self.env, probe, tracked.as_ref());
                self.read_local(probe, &answer);
                answer
            }
        };
        match answer {
            Some((class, data)) => Slot::Done(self.outcome(probe, class, data)),
            None => {
                self.effects.count_not_run(&one.call.checker, 1);
                reading.not_run = true;
                Slot::Stopped
            }
        }
    }

    /// A network probe: gated, then one job per digest for its extension.
    fn network(
        &mut self,
        one: &Asked<'a>,
        probe: &'a Probe,
        scopes: &'a [String],
        reading: &mut Reading,
    ) -> Slot {
        let Some((request, host, max_bytes)) = request_of(probe) else {
            refuse(
                reading,
                one.value,
                Diagnostic::new(
                    codes::E090,
                    format!(
                        "extension '{}': `{}` names no host",
                        one.call.extension,
                        one_probe(probe)
                    ),
                ),
            );
            return Slot::Stopped;
        };
        let extension = one.call.extension.as_str();
        let Some(gate) = self.gates.get(extension) else {
            return Slot::Stopped;
        };
        if let Err(denial) = gate.only(scopes).permits(OP, &request)
            && !matches!(denial.why, Denied::NotApproved | Denied::Root)
        {
            refuse(reading, one.value, denial.diagnostic());
            return Slot::Stopped;
        }
        if let Some(grant) = self.effects.grant(extension)
            && let Err(denial) = grant.only(scopes).permits(OP, &request)
            && let Some(name) = denial.scope
        {
            self.uses
                .entry(extension.to_string())
                .or_default()
                .insert((OP, name));
        }
        let key = (extension, specforge_effects::digest(&request));
        let next = self.jobs.len();
        let job = *self.by_digest.entry(key).or_insert(next);
        if job == next {
            self.jobs.push(Job {
                extension,
                scopes,
                scope: &one.call.checker,
                probe,
                request,
                host,
                max_bytes,
            });
        }
        Slot::Job(job)
    }

    /// `answer`, what a job's request came to, as `one`'s. A job not performed was counted by its first asker's scope;
    /// each other value it would have answered counts too. One over the budget counts for every value it would have
    /// answered.
    fn answered(
        &self,
        one: &Asked<'_>,
        reading: &mut Reading,
        answer: &Performed,
        job: &Job<'_>,
        first: bool,
    ) -> Option<ProbeOutcome> {
        match answer {
            Performed::OverBudget => {
                self.effects.count_not_run(budget::OVER, 1);
                reading.not_run = true;
                None
            }
            Performed::Declined => {
                if !first {
                    self.effects.count_not_run(&one.call.checker, 1);
                }
                reading.not_run = true;
                None
            }
            // The grant stopped it on the way: a redirect out of its scope.
            Performed::Answer(outcome) if outcome.class == OutcomeClass::Denied => {
                refuse(
                    reading,
                    one.value,
                    Diagnostic::new(
                        codes::E090,
                        format!(
                            "extension '{}': `{}` went outside its granted scope",
                            one.call.extension,
                            one_line(&job.request)
                        ),
                    ),
                );
                None
            }
            Performed::Answer(outcome) => Some(outcome.clone()),
        }
    }
}

/// `diagnostic` on `value`: the host's, and the value fails or warns by it.
fn refuse(reading: &mut Reading, value: &Value<'_>, mut diagnostic: Diagnostic) {
    diagnostic.span = value.span.clone();
    diagnostic.data = Some(Box::new(specforge_common::DiagnosticData::Subject {
        entity: value.entity.to_string(),
    }));
    reading.refused_class = Some(OutcomeClass::Denied);
    reading.refused.push(diagnostic);
}

/// Where `value` writes its root's name, when its text holds it as written; else the value.
fn root_span(value: &Value<'_>) -> Option<SourceSpan> {
    let span = value.span.clone()?;
    let at = match RootedPath::parse(value.text, Root::Project) {
        Ok(rooted) => rooted.written_root,
        Err(PrefixProblem::BadRootName { at }) => Some(at),
        Err(_) => None,
    };
    match (at, value.inner) {
        (Some(at), Some((line, column))) => Some(SourceSpan {
            start_line: line,
            start_col: column + at.start,
            end_line: line,
            end_col: column + at.end,
            ..span
        }),
        _ => Some(span),
    }
}

/// A probe in one line, for a person.
fn one_probe(probe: &Probe) -> String {
    match probe {
        Probe::GitObject { rev } => format!("object {rev}"),
        Probe::GitRef { name } => format!("ref {name}"),
        Probe::GitTracked { path, .. } => format!("tracked {path}"),
        Probe::HttpHead { url, .. } | Probe::HttpGet { url, .. } => url.clone(),
        Probe::HttpTls { host, port } => format!("tls {host}:{port}"),
        Probe::DnsResolve { host } => format!("resolve {host}"),
        Probe::FsStat { path, .. }
        | Probe::FsReadPrefix { path, .. }
        | Probe::FsRead { path, .. }
        | Probe::FsWalk { path, .. } => path.clone(),
    }
}

/// The request a network probe makes, the host it reaches, and the most body it keeps; what only the host may set is
/// set here. None for a URL with no host, which no request can reach.
fn request_of(probe: &Probe) -> Option<(EffectRequest, String, u64)> {
    let timeout_ms = DEFAULT_TIMEOUT.as_millis() as u64;
    let http = |method: &str, url: &str, redirects: u32, headers, max_bytes: u64| {
        let host = http::host_of(url)?;
        Some((
            EffectRequest::Http(HttpRequest {
                method: method.to_string(),
                url: url.to_string(),
                headers,
                body_cap: max_bytes + HEAD_ROOM,
                timeout_ms,
                max_redirects: redirects.min(http::MAX_REDIRECTS),
                keep_headers: KEPT_HEADERS.map(String::from).to_vec(),
                ..HttpRequest::default()
            }),
            host,
            max_bytes,
        ))
    };
    match probe {
        Probe::HttpHead { url, redirects } => http("HEAD", url, *redirects, Vec::new(), 0),
        Probe::HttpGet {
            url,
            range,
            max_bytes,
        } => {
            let headers = range
                .map(|[from, to]| vec![Header::plain("range", format!("bytes={from}-{to}"))])
                .unwrap_or_default();
            http(
                "GET",
                url,
                http::MAX_REDIRECTS,
                headers,
                u64::from((*max_bytes).min(BODY_CAP)),
            )
        }
        Probe::HttpTls { host, port } => Some((
            EffectRequest::Tls(TlsRequest {
                host: host.clone(),
                tcp_port: *port,
                allow_private: false,
                timeout_ms,
            }),
            host.clone(),
            0,
        )),
        Probe::DnsResolve { host } => Some((
            EffectRequest::Resolve(ResolveRequest {
                host: host.clone(),
                allow_private: false,
                timeout_ms,
            }),
            host.clone(),
            0,
        )),
        _ => None,
    }
}
