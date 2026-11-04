//! What a checker's probes ask, performed by the host (ADR 0336): each path contained, each request gated on the
//! checker's scopes, one request per digest, and the network's through the run's effects. A probe not performed leaves
//! its value not judged, so nothing goes red offline.

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use specforge_common::remedy::printable;
use specforge_common::{Diagnostic, SourceSpan, codes};
use specforge_effects::contained::{Contained, Refused, Resolved};
use specforge_effects::grant::{Denied, Granted, Uses, one_line};
use specforge_effects::{
    EffectRequest, Effects, EffectsMode, FsOp, FsRequest, Grant, NoConsent, OutcomeClass, Roots,
    UnixMillis,
};
use specforge_protocol_types::{
    ByteRange, Hosts, MAX_CAPABILITY_BYTES, Operation, OutcomeData, PortScope, PrefixProblem,
    Probe, ProbeOutcome, Root, RootedPath,
};
use specforge_registry::{CheckCall, Disk, DiskLookup};

use super::budget::{self, Spent, ValueBudget};
use super::collect::Value;
use super::local::{self, GitPath, WALK_CAP, git_of, stat_of};
use super::network::{BODY_CAP, Job, Performed, Spending, perform_all, request_of};
use crate::Environment;

/// The kind of call a probe is asked under.
const OP: Operation = Operation::CheckValues;
/// The most of a file `fs.read` answers for a prefix and for any other range, and of a body `http.get` does.
const PREFIX_CAP: u32 = 64 * 1024;
const READ_CAP: u32 = 1024 * 1024;

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

/// One probe on its way: who asked it, under which scopes, and the reading its value is building.
struct Turn<'t, 'a> {
    one: &'t Asked<'a>,
    probe: &'a Probe,
    scopes: &'a [String],
    reading: &'t mut Reading,
}

/// What containment made of a written path.
enum Held {
    /// A refusal or a warning the host reported on the value: nothing reads it.
    Reported,
    /// A bare path that names nothing.
    Nothing,
    At(Resolved),
}

/// A probe's place in its [`Reading`].
enum Slot {
    Done(ProbeOutcome),
    Job(usize),
    /// Refused or not performed: the reading says so.
    Stopped,
}

/// Perform what `asked` asks through `effects` and `disk`, within what the run has not `spent`: a reading for each. What
/// these values use of the scopes this machine has not approved joins `spent.uses`: only those are asked for.
pub(super) fn dispatch(
    env: &Environment,
    effects: &Effects,
    disk: &dyn Disk,
    asked: &[Asked<'_>],
    spent: &mut Spent,
) -> Vec<Reading> {
    let roots = env
        .effects_allowed()
        .with_named(Roots::new(env.root(), env.spec_root()));
    let lookup = DiskLookup(disk);
    let budget = env.value_budget();
    let deadline = spent.deadline;
    let mut probing = Probing {
        env,
        effects,
        disk,
        budget,
        deadline,
        spent: spent.bytes,
        read_before: spent.local,
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
    let fetched: u64 = answers
        .iter()
        .filter_map(|answer| match answer {
            Performed::Answer(outcome) => Some(budget::cost(&outcome.data)),
            _ => None,
        })
        .sum();
    spent.bytes = probing.spent + fetched;
    spent.local += probing.local.len();
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
    for (extension, uses) in std::mem::take(&mut probing.uses) {
        spent.uses.entry(extension).or_default().extend(uses);
    }
    readings
}

/// W170 for what this machine has not approved of the scopes the run's values used, one per extension.
pub(super) fn approvals(effects: &Effects, spent: &Spent) -> Vec<Diagnostic> {
    spent
        .uses
        .iter()
        .filter_map(|(extension, uses)| effects.grant(extension)?.unapproved_summary(uses))
        .collect()
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
    /// What the run read so far (the budget's `bytes`).
    spent: u64,
    /// The distinct file and git probes earlier rounds read.
    read_before: usize,
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
            || (self.read_before + self.local.len() < self.budget.local_probes
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
        let mut turn = Turn {
            one,
            probe,
            scopes,
            reading,
        };
        match probe {
            Probe::FsStat { root, path } => self.fs(&mut turn, root, path, FsOp::Stat),
            Probe::FsRead { root, path, range } => {
                let max_bytes = held(*range).len();
                self.fs(&mut turn, root, path, FsOp::Read { max_bytes })
            }
            Probe::FsDigest { root, path } => self.fs(&mut turn, root, path, FsOp::Digest),
            Probe::FsWalk { root, path, glob } => {
                let op = FsOp::Walk {
                    glob: Some(glob.clone()),
                    max_entries: WALK_CAP as u32,
                };
                self.fs(&mut turn, root, path, op)
            }
            Probe::GitObject { rev } => self.git(&mut turn, rev, None),
            Probe::GitRef { name } => self.git(&mut turn, name, None),
            Probe::GitFile { rev, path } => self.git(&mut turn, rev, Some(GitPath::FromTop(path))),
            Probe::GitTracked { root, path } => {
                self.git(&mut turn, "HEAD", Some(GitPath::Under((root, path))))
            }
            Probe::HttpHead { .. }
            | Probe::HttpGet { .. }
            | Probe::HttpPostQuery { .. }
            | Probe::HttpTls { .. }
            | Probe::DnsResolve { .. } => self.network(&mut turn),
        }
    }

    /// E094, E095, W174 or W175 for `written` under `default`, reported by the host on the value; else where it is.
    fn contain(
        &self,
        one: &Asked<'_>,
        default: &Root,
        written: &str,
        reading: &mut Reading,
    ) -> Held {
        let value = one.value;
        let shown = printable(written);
        let at = |code, message: String, suggestion: String, span| {
            let mut diagnostic = Diagnostic::new(code, message).with_suggestion(suggestion);
            value.places(&mut diagnostic, span);
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
            Err(Refused::Empty) => return Held::Nothing,
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
            Ok(resolved) => return Held::At(resolved),
        };
        // A warning leaves the value unread, so not judged; an escape fails it.
        if reading.refused_class.is_none() {
            self.effects.count_not_run(&one.call.checker, 1);
        }
        reading.refused.push(refused);
        Held::Reported
    }

    /// Whether the checker's scopes reach `request`: E090 when they do not, in every mode. A root this machine has not
    /// approved is not performed, and asks for its approval.
    fn gated_fs(&mut self, turn: &mut Turn<'_, 'a>, request: &EffectRequest) -> bool {
        let Turn {
            one,
            scopes,
            reading,
            ..
        } = turn;
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
                        if let Some(name) = denial.capability {
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
    fn fs(&mut self, turn: &mut Turn<'_, 'a>, root: &Root, written: &str, op: FsOp) -> Slot {
        let resolved = match self.contain(turn.one, root, written, turn.reading) {
            Held::Reported => return Slot::Stopped,
            Held::Nothing => None,
            Held::At(resolved) => Some(resolved),
        };
        let (rooted, rel) = match &resolved {
            Some(resolved) => (resolved.root.as_str().to_string(), resolved.rel.clone()),
            None => (root.to_string(), String::new()),
        };
        let request = EffectRequest::Fs(FsRequest {
            root: rooted,
            path: rel,
            op: op.clone(),
        });
        if !self.gated_fs(turn, &request) {
            return Slot::Stopped;
        }
        if self.effects.mode() == EffectsMode::Replay {
            self.effects.count_not_run(&turn.one.call.checker, 1);
            turn.reading.not_run = true;
            return Slot::Stopped;
        }
        let probe = turn.probe;
        if !self.local_budget(turn.reading, probe) {
            return Slot::Stopped;
        }
        let answer = match self.local.get(probe) {
            Some(answer) => answer.clone(),
            None => {
                let answer = Some(match &resolved {
                    Some(resolved) if resolved.withheld => {
                        (OutcomeClass::Denied, OutcomeData::None)
                    }
                    Some(resolved) => {
                        let range = match probe {
                            Probe::FsRead { range, .. } => Some(held(*range)),
                            _ => None,
                        };
                        let disk = self.disk;
                        local::read_disk(disk, &self.contained, resolved, &op, range, self.deadline)
                    }
                    None => (OutcomeClass::Missing, stat_of(None, 0, None)),
                });
                self.read_local(probe, &answer);
                answer
            }
        };
        let (class, data) = answer.expect("a file probe always answers");
        if class == OutcomeClass::Deadline {
            self.effects.count_not_run(budget::OVER, 1);
            turn.reading.not_run = true;
            return Slot::Stopped;
        }
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
        turn: &mut Turn<'_, 'a>,
        revision: &str,
        written: Option<GitPath<'_, (&Root, &str)>>,
    ) -> Slot {
        let Turn {
            one,
            probe,
            scopes,
            reading,
        } = turn;
        let probe: &'a Probe = probe;
        let path = match written {
            None => None,
            Some(GitPath::FromTop(path)) => Some(GitPath::FromTop(path)),
            Some(GitPath::Under((root, path))) => match self.contain(one, root, path, reading) {
                Held::At(resolved) => Some(GitPath::Under(resolved)),
                Held::Reported => return Slot::Stopped,
                Held::Nothing => {
                    return Slot::Done(self.outcome(
                        probe,
                        OutcomeClass::Missing,
                        git_of(false, None),
                    ));
                }
            },
        };
        let question = one_probe(probe);
        let Some(gate) = self.gates.get(&one.call.extension) else {
            return Slot::Stopped;
        };
        if let Err(denial) = gate.only(scopes).permits_git(OP, &question) {
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
                let answer = local::ask_git(self.env, revision, path);
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
    fn network(&mut self, turn: &mut Turn<'_, 'a>) -> Slot {
        let Turn {
            one,
            probe,
            scopes,
            reading,
        } = turn;
        let (probe, scopes): (&'a Probe, &'a [String]) = (probe, scopes);
        let Some((request, host, max_bytes)) = request_of(probe, BODY_CAP) else {
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
        let only = gate.only(scopes);
        let permitting = match only.permitted(OP, &request) {
            Ok(granted) => Some(granted),
            Err(denial) if matches!(denial.why, Denied::NotApproved | Denied::Root) => denial
                .capability
                .and_then(|name| only.capabilities().iter().find(|g| g.declared.name == name)),
            Err(denial) => {
                refuse(reading, one.value, denial.diagnostic());
                return Slot::Stopped;
            }
        };
        // What a capability that lists its hosts lets the answer hold; none for the hosts values name.
        let listed = permitting.and_then(|granted| match &granted.declared.port {
            PortScope::Http(http) if matches!(http.hosts, Hosts::List(_)) => Some(http.max_bytes),
            _ => None,
        });
        if matches!(probe, Probe::HttpPostQuery { .. }) && listed.is_none() {
            let why = format!(
                "extension '{}': `http.post_query {}` goes only to a host its capability lists",
                one.call.extension,
                one_probe(probe)
            );
            refuse(reading, one.value, Diagnostic::new(codes::E090, why));
            return Slot::Stopped;
        }
        let room = listed
            .flatten()
            .and_then(|max| u32::try_from(max.min(MAX_CAPABILITY_BYTES)).ok())
            .unwrap_or(BODY_CAP);
        let (request, host, max_bytes) = match room {
            BODY_CAP => (request, host, max_bytes),
            room => request_of(probe, room).unwrap_or((request, host, max_bytes)),
        };
        if let Some(grant) = self.effects.grant(extension)
            && let Err(denial) = grant.only(scopes).permits(OP, &request)
            && let Some(name) = denial.capability
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
                queries: matches!(probe, Probe::HttpPostQuery { .. })
                    && permitting.is_some_and(Granted::queries),
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

/// `range` held to what a file read answers: a prefix to 64 KiB, any other range to 1 MiB.
fn held(range: ByteRange) -> ByteRange {
    match range {
        ByteRange::Prefix { .. } => range.held_to(PREFIX_CAP),
        _ => range.held_to(READ_CAP),
    }
}

/// `diagnostic` on `value`: the host's, and the value fails or warns by it.
fn refuse(reading: &mut Reading, value: &Value<'_>, mut diagnostic: Diagnostic) {
    value.places(&mut diagnostic, value.span.clone());
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
        Probe::GitFile { rev, path } => format!("file {rev}:{path}"),
        Probe::GitTracked { path, .. } => format!("tracked {path}"),
        Probe::HttpHead { url, .. }
        | Probe::HttpGet { url, .. }
        | Probe::HttpPostQuery { url, .. } => url.clone(),
        Probe::HttpTls { host, port } => format!("tls {host}:{port}"),
        Probe::DnsResolve { host } => format!("resolve {host}"),
        Probe::FsStat { path, .. }
        | Probe::FsRead { path, .. }
        | Probe::FsDigest { path, .. }
        | Probe::FsWalk { path, .. } => path.clone(),
    }
}
