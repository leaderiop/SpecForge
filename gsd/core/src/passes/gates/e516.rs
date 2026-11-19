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
//! finding. Silent outside a gate. The suggestion names the remedy by the rule's judge: `re_verify` lists the
//! verifier's verdicts only, so another judge's verdict is recorded again by the step that runs it.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::{gate, phases, plans_in, refuse};
use crate::passes::commits::target;
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

/// How the verdict of `rule` on `entity`, on record and no longer usable, is recorded again over its current inputs,
/// by its judge (`judge`, `None` for another extension's rule): the verifier's are listed by `re_verify` over the
/// entity's plan (its `plan` edge); another core rule's judge records it in the step that runs it; another
/// extension's rule is named by the inputs query, which prints its judge.
fn again(input: &PassInput, entity: &str, rule: &str, judge: Option<&str>) -> String {
    match judge {
        Some(VERIFIER) => format!(
            "record the verdict again over the current inputs: `specforge gsd re-verify {} --stale-only` lists the \
             verifier's verdicts that are not fresh",
            target(input, entity, "plan").unwrap_or("<its plan>")
        ),
        Some(judge) => format!(
            "run the step that judges it again (its judge: {judge}): it records the verdict over the current inputs \
             (`specforge query {entity} --inputs {rule}`)"
        ),
        None => format!(
            "run the step of its extension that judges it again: it records the verdict over the current inputs \
             (`specforge query {entity} --inputs {rule}` names its judge)"
        ),
    }
}

/// How a missing verdict of `rule` on `entity` is given: the step that judges it, named by its judge for a core rule
/// and by the inputs query for another extension's.
fn first(entity: &str, rule: &str, judge: Option<&str>) -> String {
    match judge {
        Some(judge) => format!("run the step that judges it (its judge: {judge})"),
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
    let mut found = Vec::new();
    for entity in &input.entities {
        let inside = within(covered.as_ref(), &entity.id);
        for j in &entity.judgements {
            let judge = core_judge(&j.rule);
            let (message, how) = match j.state.as_str() {
                "stale" => (
                    if j.changed.is_empty() {
                        "is stale".to_string()
                    } else {
                        format!("is stale: {} changed", j.changed.join(", "))
                    },
                    again(input, &entity.id, &j.rule, judge),
                ),
                "unrecorded" => (
                    "is not recorded in specforge-harness".to_string(),
                    again(input, &entity.id, &j.rule, judge),
                ),
                "unreadable" => (
                    "cannot be read".to_string(),
                    "fix what the inputs cannot read, then record the verdict again".to_string(),
                ),
                "missing" if inside => {
                    ("is missing".to_string(), first(&entity.id, &j.rule, judge))
                }
                _ => continue,
            };
            if inside && judge.is_some_and(|judge| judges.contains(judge)) {
                continue;
            }
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
