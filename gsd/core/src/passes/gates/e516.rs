//! E516: a verdict that does not hold blocks every workflow step but the one that judges it (ADR 0377).
//!
//! At a gate, each judgement owed now (the host settles away the rest) whose state is `stale`, `unrecorded`,
//! `unreadable` or `missing` is one error, per entity and rule. A stale, unrecorded or unreadable verdict is on record
//! and no longer usable, so it refuses every gate wherever it is; a missing one is not yet given, which is the gate's
//! business only within its scope (the subject, its phases and plans, and every entity they scope; the whole project
//! without a subject). Within that scope a judgement of a core rule whose judge the gated workflow runs itself (one of
//! its agent steps, or the workflow as `prompt:<id>`) is excused: that workflow is the verdict's remedy. A workflow it
//! continues with runs behind its own gate, so it excuses nothing here. A rule of another extension is never excused
//! by a core workflow. A fresh verdict, passing or failing, is never E516: a failing outcome is the rule's own
//! finding. Silent outside a gate.
//!
//! The suggestion of a core rule's verdict names the prompts whose run records its judge's verdicts again (each
//! workflow's `rejudges`), with the flags the run needs, on the phase or milestone that holds the verdict. A run
//! excuses a verdict only inside its own scope and only for a judge it runs, so the suggestion also names every
//! verdict, of any judge or extension, that refuses every run it names, by its holder, its judge and the entities
//! that owe it (each once, by its full id, as the message writes it; at most five per holder and judge, then how many
//! more), and the gate-free `record-verdict`; a verdict no phase or milestone holds is named after the holders by its
//! own entity, owed by it itself. With no such run (no phase or milestone scopes the entity, or the prompt stops on
//! its complete phase) it names `record-verdict` alone. `re_verify` lists the verifier's verdicts only. Every id and
//! name the message and the help echo is [`one_line`], each id in the message and the tail [`quoted`] alike.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::{gate, phases, plans_in, refuse};
use crate::commands::id_order;
use crate::passes::commits::target;
use crate::passes::plans::text;
use crate::passes::{one_line, quoted};
use crate::prompts::{Rejudge, Spec, judges_run_by, rejudging};
use crate::scope::scope_of;

/// The ids whose scope a gate covers: the subject, its phases and their plans (a milestone's phases are linked, not
/// scoped by path), `None` for the whole project.
fn covered(input: &PassInput, subject: Option<&str>) -> Option<BTreeSet<String>> {
    let subject = subject?;
    let mut ids = BTreeSet::from([subject.to_string()]);
    ids.extend(
        phases(input, Some(subject))
            .into_iter()
            .map(|p| p.id.clone()),
    );
    ids.extend(
        plans_in(input, Some(subject))
            .into_iter()
            .map(|p| p.id.clone()),
    );
    Some(ids)
}

/// Whether `id` is within the gate's scope: it, or a scope ancestor of it, is a covered id.
fn within(covered: Option<&BTreeSet<String>>, id: &str) -> bool {
    let Some(covered) = covered else {
        return true;
    };
    let mut at = id;
    while !at.is_empty() {
        if covered.contains(at) {
            return true;
        }
        at = scope_of(at);
    }
    false
}

/// The local code and judge of a core rule (`gsd/E501` is gsd-verifier's `E501`); `None` for another extension's rule.
fn core(rule: &str) -> Option<(&str, &'static str)> {
    let local = rule.strip_prefix("gsd/")?;
    crate::judged::get(local).map(|j| (local, j.judge))
}

/// The judge whose verdicts `re_verify` lists.
const VERIFIER: &str = "gsd-verifier";

/// How many of the entities owing one holder's verdicts to one judge the tail names; the count says how many more.
const NAMED: usize = 5;

/// Whether a judgement on record refuses a gate whose workflow runs the judges `runs`: a stale, unrecorded or
/// unreadable verdict wherever it is, a `missing` one only `inside` the gate's scope, unless it is inside and the
/// workflow runs its core `judge` (that workflow is its remedy). The gate and every run its suggestion names read it.
fn refuses(inside: bool, missing: bool, runs: &BTreeSet<String>, judge: Option<&str>) -> bool {
    (inside || !missing) && !(inside && judge.is_some_and(|judge| runs.contains(judge)))
}

/// A prompt whose run records a judge's verdicts again: the workflow, what its run needs, and the kind its subject
/// references (one kind: the registry's tests hold it).
#[derive(Clone, Copy)]
struct Remedy {
    spec: &'static Spec,
    rejudge: &'static Rejudge,
    kind: &'static str,
}

/// A judgement on record that refuses some gate: the entity, its rule, the rule's core judge (`None` for another
/// extension's) and whether it is only missing.
struct Owing<'a> {
    entity: &'a str,
    rule: &'a str,
    judge: Option<&'static str>,
    missing: bool,
}

/// The group a verdict that refuses a named run falls in: the phase or milestone that holds it, else the outermost id
/// it falls back to, and its judge (another extension's rule when it has no core judge).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Blocker<'a> {
    holder: &'a str,
    by: &'a str,
}

/// The verdicts that refuse a run, per holder and judge: the entities that owe them.
type Owed<'a> = BTreeMap<Blocker<'a>, BTreeSet<&'a str>>;

/// The entities of one group of a tail: the first [`NAMED`] in `id_order`, each [`quoted`] as the message writes its
/// own, then how many more. Each id's order key is read once and only the first [`NAMED`] are sorted: the key holds the
/// id, so the order is total and the selection exact.
fn names(entities: &BTreeSet<&str>) -> String {
    let mut keyed: Vec<_> = entities.iter().map(|id| id_order(id)).collect();
    if keyed.len() > NAMED {
        keyed.select_nth_unstable(NAMED);
        keyed.truncate(NAMED);
    }
    keyed.sort_unstable();
    let more = match entities.len().saturating_sub(NAMED) {
        0 => String::new(),
        n => format!(", and {n} more"),
    };
    let named: Vec<String> = keyed.iter().map(|(_, id)| quoted(id)).collect();
    format!("{}{more}", named.join(", "))
}

/// Prompt runs: each remedy with the subject it runs on.
type Runs<'a> = Vec<(Remedy, &'a str)>;

/// A prompt run as the index keys what it reads of it: the prompt's id and the subject.
type Run<'a> = (&'static str, &'a str);

/// `/gsd-<command> <at> <flags>` and its CLI form: one prompt run a suggestion names.
fn item(remedy: &Remedy, at: &str) -> String {
    let run = std::iter::once(one_line(at))
        .chain(
            remedy
                .rejudge
                .flags
                .iter()
                .map(|flag| format!("--{}", flag.replace('_', "-"))),
        )
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "/gsd-{} {run} (`specforge prompt gsd.{} {run}`)",
        remedy.spec.command, remedy.spec.id
    )
}

/// What the suggestions of one gate read, built at its first refusal: every entity by id, the prompts that record each
/// core judge's verdicts again (read from the registry once), every judgement that refuses some gate, per named run
/// the verdicts that refuse it, and per set of named runs its tail.
struct Index<'a> {
    input: &'a PassInput,
    entities: BTreeMap<&'a str, &'a PassEntity>,
    remedies: BTreeMap<&'static str, Vec<Remedy>>,
    /// The kinds a remedy prompt runs on (`phase`, `milestone`): what holds a verdict.
    holders: BTreeSet<&'static str>,
    owing: Vec<Owing<'a>>,
    /// By `(prompt, subject)`: the verdicts that refuse its run, per holder and judge, with the entities owing them.
    blocking: BTreeMap<Run<'a>, Owed<'a>>,
    /// By the runs a help names: its tail, which depends on nothing else, so every diagnostic naming them shares it.
    tails: BTreeMap<Vec<Run<'a>>, String>,
}

impl<'a> Index<'a> {
    fn new(input: &'a PassInput) -> Self {
        let judges: BTreeSet<&'static str> =
            crate::judged::JUDGED.iter().map(|j| j.judge).collect();
        let remedies: BTreeMap<&'static str, Vec<Remedy>> = judges
            .into_iter()
            .map(|judge| {
                let found = rejudging(judge)
                    .into_iter()
                    .filter_map(|(spec, rejudge)| {
                        Some(Remedy {
                            spec,
                            rejudge,
                            kind: spec.subject_kind()?,
                        })
                    })
                    .collect();
                (judge, found)
            })
            .collect();
        let holders = remedies.values().flatten().map(|r| r.kind).collect();
        let owing = input
            .entities
            .iter()
            .flat_map(|e| {
                e.judgements.iter().filter_map(move |j| {
                    let missing = match j.state.as_str() {
                        "missing" => true,
                        "stale" | "unrecorded" | "unreadable" => false,
                        _ => return None,
                    };
                    Some(Owing {
                        entity: e.id.as_str(),
                        rule: j.rule.as_str(),
                        judge: core(&j.rule).map(|(_, judge)| judge),
                        missing,
                    })
                })
            })
            .collect();
        Index {
            input,
            entities: input.entities.iter().map(|e| (e.id.as_str(), e)).collect(),
            remedies,
            holders,
            owing,
            blocking: BTreeMap::new(),
            tails: BTreeMap::new(),
        }
    }

    /// The kind of the entity `id`, when there is one.
    fn kind(&self, id: &str) -> Option<&'a str> {
        self.entities.get(id).map(|e| e.kind.as_str())
    }

    /// The entity of kind `kind` that scopes `entity`, itself included, climbing `scope_of`: the phase of a truth, plan
    /// or phase spec, the milestone of an audit.
    fn subject_of(&self, entity: &'a str, kind: &str) -> Option<&'a str> {
        let mut at = entity;
        while !at.is_empty() {
            if self.kind(at) == Some(kind) {
                return Some(at);
            }
            at = scope_of(at);
        }
        None
    }

    /// Whether `id` is an entity of a kind a remedy prompt runs on (a phase, a milestone): one that holds the verdicts
    /// it scopes. [`Index::holder`] and the tail read it.
    fn holds(&self, id: &str) -> bool {
        self.kind(id)
            .is_some_and(|kind| self.holders.contains(kind))
    }

    /// The phase or milestone that holds a verdict on `entity`, else its outermost scope (a quick plan, the project's
    /// research), which holds nothing.
    fn holder(&self, entity: &'a str) -> &'a str {
        let (mut at, mut outermost) = (entity, entity);
        while !at.is_empty() {
            if self.holds(at) {
                return at;
            }
            outermost = at;
            at = scope_of(at);
        }
        outermost
    }

    /// The runs that record `judge`'s verdict on `entity` again, each prompt on the subject that holds it; apart, the
    /// prompts left out because they stop on that complete phase.
    fn runs(&self, entity: &'a str, judge: &str) -> (Runs<'a>, Runs<'a>) {
        let (mut named, mut stopped) = (Vec::new(), Vec::new());
        for &remedy in self.remedies.get(judge).into_iter().flatten() {
            let Some(at) = self.subject_of(entity, remedy.kind) else {
                continue;
            };
            let complete =
                self.entities.get(at).and_then(|e| text(e, "status")) == Some("complete");
            if remedy.rejudge.stops_on_complete && complete {
                stopped.push((remedy, at));
            } else {
                named.push((remedy, at));
            }
        }
        (named, stopped)
    }

    /// The verdicts that refuse a run of `remedy` on `at`, as the gate of that run would read them: owed elsewhere,
    /// or owed inside its scope to a judge or extension it does not run; per holder and judge, the entities that owe
    /// them. Computed once per run.
    fn blockers(&mut self, remedy: Remedy, at: &'a str) -> &Owed<'a> {
        let key = (remedy.spec.id, at);
        if !self.blocking.contains_key(&key) {
            let covered = covered(self.input, Some(at));
            let runs = judges_run_by(remedy.spec.id);
            let mut found = Owed::new();
            for o in self.owing.iter().filter(|o| {
                refuses(
                    within(covered.as_ref(), o.entity),
                    o.missing,
                    &runs,
                    o.judge,
                )
            }) {
                let blocker = Blocker {
                    holder: self.holder(o.entity),
                    by: o.judge.unwrap_or(o.rule),
                };
                found.entry(blocker).or_default().insert(o.entity);
            }
            self.blocking.insert(key, found);
        }
        &self.blocking[&key]
    }

    /// The tail naming the verdicts that refuse every run of `named`, in natural order, by their holder, their judge
    /// and the entities that owe them, each once: each sits outside the run's scope or waits for a judge or extension
    /// the run does not run. An entity is named only when its verdict refuses every run, and a group with none left is
    /// left out. The groups a phase or milestone holds come first, behind one `on`; then each group whose outermost id
    /// holds nothing, its entities named by their own ids, each owing the verdict itself. Empty when no group is left.
    /// Built once per set of runs: the gate's diagnostics that name the same runs share it.
    fn tail(&mut self, named: &[(Remedy, &'a str)]) -> String {
        let key: Vec<Run<'a>> = named.iter().map(|(r, at)| (r.spec.id, *at)).collect();
        if let Some(tail) = self.tails.get(&key) {
            return tail.clone();
        }
        let tail = self.build_tail(named);
        self.tails.insert(key, tail.clone());
        tail
    }

    /// The tail of [`Index::tail`], built.
    fn build_tail(&mut self, named: &[(Remedy, &'a str)]) -> String {
        let mut common: Option<Owed<'a>> = None;
        for &(remedy, at) in named {
            let found = self.blockers(remedy, at);
            common = Some(match common {
                None => found.clone(),
                Some(common) => common
                    .into_iter()
                    .filter_map(|(blocker, entities)| {
                        let both: BTreeSet<&'a str> = entities
                            .intersection(found.get(&blocker)?)
                            .copied()
                            .collect();
                        (!both.is_empty()).then_some((blocker, both))
                    })
                    .collect(),
            });
        }
        let mut groups: Vec<(Blocker<'a>, BTreeSet<&'a str>)> =
            common.unwrap_or_default().into_iter().collect();
        if groups.is_empty() {
            return String::new();
        }
        groups.sort_by_cached_key(|(b, _)| (id_order(b.holder), b.by));
        let (held, unheld): (Vec<_>, Vec<_>) =
            groups.iter().partition(|(b, _)| self.holds(b.holder));
        let mut parts: Vec<String> = Vec::new();
        if !held.is_empty() {
            let listed: Vec<String> = held
                .iter()
                .map(|(b, entities)| {
                    format!(
                        "{} ({}: {})",
                        one_line(b.holder),
                        one_line(b.by),
                        names(entities)
                    )
                })
                .collect();
            parts.push(format!("on {}", listed.join(", ")));
        }
        parts.extend(unheld.iter().map(|(b, entities)| {
            let owing = if entities.len() == 1 {
                " itself"
            } else {
                ", each itself"
            };
            format!("by {}{owing} ({})", names(entities), one_line(b.by))
        }));
        format!(
            "; every run named here is also refused by the verdicts owed {}, outside its scope or of a judge it does \
             not run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no gate refuses",
            parts.join(", and ")
        )
    }

    /// How the verdict of `rule` on `entity`, in state `state`, is given or recorded again: by its judge's prompts on
    /// the phase or milestone holding it, with the verdicts that refuse each of those runs; by `record-verdict` when no
    /// prompt's run can; by the step of its extension for another extension's rule (`core` is `None`), which the inputs
    /// query names.
    fn suggestion(
        &mut self,
        entity: &'a str,
        rule: &str,
        core: Option<(&str, &'static str)>,
        state: &str,
    ) -> String {
        let missing = state == "missing";
        if state == "unreadable" {
            return "fix what the inputs cannot read, then record the verdict again".to_string();
        }
        // The entity and rule as the help echoes them; `entity` itself is the key every lookup reads.
        let (id, rule) = (one_line(entity), one_line(rule));
        let Some((code, judge)) = core else {
            return if missing {
                format!(
                    "run the step of its extension that judges it (`specforge query {id} --inputs {rule}` names its \
                     judge)"
                )
            } else {
                format!(
                    "run the step of its extension that judges it again: it records the verdict over the current \
                     inputs (`specforge query {id} --inputs {rule}` names its judge)"
                )
            };
        };
        let code = one_line(code);
        let (named, stopped) = self.runs(entity, judge);
        if named.is_empty() {
            let again = if missing { "" } else { " again" };
            let why = match stopped.first() {
                Some((_, at)) => {
                    let commands: Vec<String> = stopped
                        .iter()
                        .map(|(r, _)| format!("/gsd-{}", r.spec.command))
                        .collect();
                    let verb = if commands.len() == 1 { "stops" } else { "stop" };
                    format!(
                        "on the complete phase {}, where {} {verb}",
                        one_line(at),
                        commands.join(" and ")
                    )
                }
                None => "where no phase or milestone scopes it".to_string(),
            };
            return format!(
                "no prompt judges it{again} {why}: record its verdict with `specforge gsd record-verdict {id} \
                 {code} <outcome> <model> '<basis>'` over its inputs (`specforge query {id} --inputs {rule}`), \
                 which no gate refuses (its judge: {judge})"
            );
        }
        let items = named
            .iter()
            .map(|(remedy, at)| item(remedy, at))
            .collect::<Vec<_>>()
            .join(" or ");
        let tail = self.tail(&named);
        if missing {
            return format!("run {items} to judge it (its judge: {judge}){tail}");
        }
        if judge == VERIFIER {
            let listing = target(self.input, entity, "plan")
                .map(|plan| {
                    format!(
                        "; `specforge gsd re-verify {} --stale-only` lists the verifier's verdicts that are not fresh",
                        one_line(plan)
                    )
                })
                .unwrap_or_default();
            return format!(
                "record the verdict again over the current inputs: run {items}{listing}{tail}"
            );
        }
        format!(
            "run {items} to judge it again (its judge: {judge}): it records the verdict over the current inputs \
             (`specforge query {id} --inputs {rule}`){tail}"
        )
    }
}

pub(super) fn e516(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(subject) = gate(input) else {
        return Vec::new();
    };
    let workflow = input
        .trigger
        .as_ref()
        .and_then(|t| t.command.as_deref())
        .unwrap_or_default();
    let runs = judges_run_by(workflow);
    let covered = covered(input, subject);
    // Built at the first refusal: a gate nothing refuses reads no registry.
    let mut index: Option<Index<'_>> = None;
    let mut found = Vec::new();
    for entity in &input.entities {
        let inside = within(covered.as_ref(), &entity.id);
        for j in &entity.judgements {
            let message = match j.state.as_str() {
                "stale" if j.changed.is_empty() => "is stale".to_string(),
                "stale" => {
                    let changed: Vec<String> = j.changed.iter().map(|c| one_line(c)).collect();
                    format!("is stale: {} changed", changed.join(", "))
                }
                "unrecorded" => "is not recorded in specforge-harness".to_string(),
                "unreadable" => "cannot be read".to_string(),
                "missing" => "is missing".to_string(),
                _ => continue,
            };
            let core = core(&j.rule);
            if !refuses(
                inside,
                j.state == "missing",
                &runs,
                core.map(|(_, judge)| judge),
            ) {
                continue;
            }
            let how = index
                .get_or_insert_with(|| Index::new(input))
                .suggestion(&entity.id, &j.rule, core, &j.state);
            // The host keeps a gate diagnostic only within the subject's scope: name the subject, so a verdict
            // anywhere refuses a scoped gate. The entity stays in the message.
            found.push(refuse(
                "E516",
                PassSeverity::Error,
                subject.unwrap_or(&entity.id),
                format!(
                    "the {} verdict on {} {message}",
                    one_line(&j.rule),
                    quoted(&entity.id)
                ),
                &how,
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests;
