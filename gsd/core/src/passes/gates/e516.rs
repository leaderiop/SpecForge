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
//! finding. Silent outside a gate. The suggestion of a core rule's verdict names the prompts that run its judge
//! (`prompts_running`), each on the phase or milestone that holds the verdict, and every other holder of the same
//! judge's owed verdicts: a prompt excuses a verdict only inside its own scope, so running it on the gated subject does
//! not clear a verdict held elsewhere, and a run on one holder is refused by the others'. `re_verify` lists the
//! verifier's verdicts only.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::{gate, phases, plans_in, refuse};
use crate::passes::commits::target;
use crate::prompts::prompts_running;
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

/// The judge of a core rule (`gsd/E501` is gsd-verifier's); `None` for another extension's rule.
fn core_judge(rule: &str) -> Option<&'static str> {
    let local = rule.strip_prefix("gsd/")?;
    crate::judged::get(local).map(|j| j.judge)
}

/// The judge whose verdicts `re_verify` lists.
const VERIFIER: &str = "gsd-verifier";

/// Each entity's kind, by id: what `subject_of` climbs through, built once per gate.
type Kinds<'a> = BTreeMap<&'a str, &'a str>;

fn kinds(input: &PassInput) -> Kinds<'_> {
    input
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.kind.as_str()))
        .collect()
}

/// The entity of kind `kind` that scopes `entity`, itself included, climbing `scope_of` to the empty scope: the phase
/// of a truth, plan or phase spec, the milestone of a phase or an audit.
fn subject_of<'e>(kinds: &Kinds, entity: &'e str, kind: &str) -> Option<&'e str> {
    let mut at = entity;
    while !at.is_empty() {
        if kinds.get(at) == Some(&kind) {
            return Some(at);
        }
        at = scope_of(at);
    }
    None
}

/// The subject kinds of the prompts that run `judge`. A prompt's `Spec.subject` is an argument name equal to the entity
/// kind for every current workflow (`phase`, `milestone`), which is why it is read as a kind.
fn subject_kinds(judge: &str) -> BTreeSet<&'static str> {
    prompts_running(judge)
        .into_iter()
        .filter_map(|s| s.subject)
        .collect()
}

/// The prompts that give or renew `judge`'s verdict on `entity`, each on the phase or milestone that holds it (`<its
/// phase>` when none scopes it), as its slash command and its CLI form, joined by ` or `; `None` when no prompt runs
/// the judge. A prompt without a slash binding is named by its CLI form alone.
fn remedies(kinds: &Kinds, entity: &str, judge: &str) -> Option<String> {
    let items: Vec<String> = prompts_running(judge)
        .into_iter()
        .filter_map(|spec| {
            let kind = spec.subject?;
            let at = subject_of(kinds, entity, kind)
                .map_or_else(|| format!("<its {kind}>"), str::to_string);
            let cli = format!("`specforge prompt gsd.{} {at}`", spec.id);
            Some(match spec.slash {
                Some(_) => format!("/gsd-{} {at} ({cli})", spec.command),
                None => cli,
            })
        })
        .collect();
    (!items.is_empty()).then(|| items.join(" or "))
}

/// The holders of each core judge's owed verdicts: the phases or milestones its prompts run on.
type Owed<'a> = BTreeMap<&'static str, BTreeSet<&'a str>>;

/// Where each core judge's verdicts are owed again: for every stale or unrecorded verdict in the project, excused by
/// the gate or not, the phase or milestone its judge's prompts run on. A missing verdict is no holder (it refuses only
/// a gate that covers it), nor is an unreadable one (it refuses every gate too, but its diagnostic carries its own
/// fix).
fn owed<'a>(input: &'a PassInput, kinds: &Kinds) -> Owed<'a> {
    let mut owed = Owed::new();
    for entity in &input.entities {
        for j in &entity.judgements {
            let Some(judge) = core_judge(&j.rule) else {
                continue;
            };
            if !matches!(j.state.as_str(), "stale" | "unrecorded") {
                continue;
            }
            owed.entry(judge).or_default().extend(
                subject_kinds(judge)
                    .into_iter()
                    .filter_map(|kind| subject_of(kinds, &entity.id, kind)),
            );
        }
    }
    owed
}

/// The tail naming every holder of `judge`'s owed verdicts but the one `entity` is in, sorted: a run on one holder is
/// refused while another's verdict is owed, since it sits outside the run's scope. Empty with one holder.
fn elsewhere(owed: &Owed, kinds: &Kinds, entity: &str, judge: &str) -> String {
    let own: BTreeSet<&str> = subject_kinds(judge)
        .into_iter()
        .filter_map(|kind| subject_of(kinds, entity, kind))
        .collect();
    let others: Vec<&str> = owed
        .get(judge)
        .into_iter()
        .flatten()
        .copied()
        .filter(|at| !own.contains(at))
        .collect();
    if others.is_empty() {
        return String::new();
    }
    format!(
        "; the same judge's verdicts are also owed on {}: each of those needs its own run, and a run is refused while \
         another holder's verdict is owed (it sits outside its scope), so record them with `specforge gsd \
         record-verdict`, which no gate refuses",
        others.join(", ")
    )
}

/// What the suggestion of a core rule's verdict names: the prompts that run its judge (`None` when none does) and the
/// tail naming the other holders.
struct Remedy {
    run: Option<String>,
    tail: String,
}

/// How the verdict of `rule` on `entity`, on record and no longer usable, is recorded again over its current inputs,
/// by its judge (`judge`, `None` for another extension's rule): the prompts that run the judge, on the phase or
/// milestone holding the verdict, and for the verifier's `re_verify` over the entity's plan (its `plan` edge); without
/// such a prompt, the step that runs the judge; another extension's rule is named by the inputs query, which prints
/// its judge.
fn again(input: &PassInput, entity: &str, rule: &str, judge: Option<(&str, Remedy)>) -> String {
    match judge {
        Some((VERIFIER, Remedy { run, tail })) => format!(
            "record the verdict again over the current inputs: {}`specforge gsd re-verify {} --stale-only` lists the \
             verifier's verdicts that are not fresh{tail}",
            run.map(|run| format!("run {run}; ")).unwrap_or_default(),
            target(input, entity, "plan").unwrap_or("<its plan>")
        ),
        Some((
            judge,
            Remedy {
                run: Some(run),
                tail,
            },
        )) => format!(
            "run {run} to judge it again (its judge: {judge}): it records the verdict over the current inputs \
             (`specforge query {entity} --inputs {rule}`){tail}"
        ),
        Some((judge, Remedy { run: None, tail })) => format!(
            "run the step that judges it again (its judge: {judge}): it records the verdict over the current inputs \
             (`specforge query {entity} --inputs {rule}`){tail}"
        ),
        None => format!(
            "run the step of its extension that judges it again: it records the verdict over the current inputs \
             (`specforge query {entity} --inputs {rule}` names its judge)"
        ),
    }
}

/// How a missing verdict of `rule` on `entity` is given: the prompts that run its judge, on the phase or milestone
/// holding it, for a core rule (the step that judges it when no prompt does), and the inputs query for another
/// extension's.
fn first(entity: &str, rule: &str, judge: Option<(&str, Remedy)>) -> String {
    match judge {
        Some((
            judge,
            Remedy {
                run: Some(run),
                tail,
            },
        )) => {
            format!("run {run} to judge it (its judge: {judge}){tail}")
        }
        Some((judge, Remedy { run: None, tail })) => {
            format!("run the step that judges it (its judge: {judge}){tail}")
        }
        None => format!(
            "run the step of its extension that judges it (`specforge query {entity} --inputs {rule}` names its judge)"
        ),
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
    let judges = crate::prompts::judges_run_by(workflow);
    let covered = covered(input, subject);
    let kinds = kinds(input);
    let owed = owed(input, &kinds);
    let mut found = Vec::new();
    for entity in &input.entities {
        let inside = within(covered.as_ref(), &entity.id);
        for j in &entity.judgements {
            let judge = core_judge(&j.rule);
            if inside && judge.is_some_and(|judge| judges.contains(judge)) {
                continue;
            }
            let remedy = || {
                judge.map(|judge| {
                    let remedy = Remedy {
                        run: remedies(&kinds, &entity.id, judge),
                        tail: elsewhere(&owed, &kinds, &entity.id, judge),
                    };
                    (judge, remedy)
                })
            };
            let (message, how) = match j.state.as_str() {
                "stale" => (
                    if j.changed.is_empty() {
                        "is stale".to_string()
                    } else {
                        format!("is stale: {} changed", j.changed.join(", "))
                    },
                    again(input, &entity.id, &j.rule, remedy()),
                ),
                "unrecorded" => (
                    "is not recorded in specforge-harness".to_string(),
                    again(input, &entity.id, &j.rule, remedy()),
                ),
                "unreadable" => (
                    "cannot be read".to_string(),
                    "fix what the inputs cannot read, then record the verdict again".to_string(),
                ),
                "missing" if inside => (
                    "is missing".to_string(),
                    first(&entity.id, &j.rule, remedy()),
                ),
                _ => continue,
            };
            // The host keeps a gate diagnostic only within the subject's scope: name the subject, so a verdict
            // anywhere refuses a scoped gate. The entity stays in the message.
            found.push(refuse(
                "E516",
                PassSeverity::Error,
                subject.unwrap_or(&entity.id),
                format!("the {} verdict on '{}' {message}", j.rule, entity.id),
                &how,
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests;
