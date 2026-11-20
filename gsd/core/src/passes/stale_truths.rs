//! The stale-truths pass: `gsd/I527`, one advisory per truth whose verifier verdict is stale, saying what changed since
//! the base the host names (ADR 0382). The stale set is `gsd/E501`'s judgements, never a digest of its own, so it
//! names exactly what `re_verify --stale-only` lists. What changed comes in two halves: the spec it reads, from the
//! graph delta of the truth, its plan and its phase; and the files it cites, from the truth's judgement at the base
//! and now, since a one-file edit is not in the delta (files are not graph nodes).
//!
//! With no base (a plain `check`, every node added) it says nothing: `gsd/E501` already reports the verdict. The files
//! half is exact only under a `revision` base whose baseline was read at the same commit; under a session's last check
//! it names the files the truth cites without claiming which changed.

use specforge_extension_sdk::prelude::*;

use super::plans::{link, of_kind, phase_of};
use crate::commands::id_order;

/// The verifier's rule, whose stale judgement on a truth is the stale set.
const RULE: &str = "gsd/E501";

/// The entities a truth's verifier verdict reads, in the order a message names them: the delta's kinds, which are the
/// entity inputs of [`TRUTH_INPUTS`](crate::judged::TRUTH_INPUTS). Its `tests` input is left out: a test is a fact,
/// named only through the judgement's changed inputs.
pub(crate) const READS: [&str; 3] = ["truth", "plan", "phase"];

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("stale_truths", |p| {
        p.phase("check")
            .baseline(&["truth"])
            .input(Input::delta(&READS))
            .run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(delta) = &input.delta else {
        return Vec::new();
    };
    let Some(base) = base_name(&delta.base) else {
        return Vec::new();
    };
    let mut stale: Vec<(&PassEntity, Verdict)> = of_kind(input, "truth")
        .filter_map(|truth| stale_verdict(truth).map(|verdict| (truth, verdict)))
        .collect();
    stale.sort_by_key(|(truth, _)| id_order(&truth.id));
    stale
        .into_iter()
        .map(|(truth, verdict)| {
            let plan = link(input, truth, "plan");
            let read = read_by(input, truth, plan);
            let spec = spec_changes(delta, &read);
            let files = changed_files(input, delta, truth, &verdict);
            let message = message(&truth.id, &verdict, &base, &spec, files.as_deref());
            let found =
                PassDiagnostic::new("I527", PassSeverity::Info, message).with_entity(&truth.id);
            match plan {
                Some(plan) => found.with_suggestion(suggestion(plan)),
                None => found,
            }
        })
        .collect()
}

/// What the pass reads of a `gsd/E501` judgement.
struct Verdict<'a> {
    /// The inputs that changed since the verdict was recorded, by name.
    changed: &'a [String],
    /// `(path, digest)` of each file its `file()` inputs and `cited` read, in the judgement's order.
    files: Vec<(&'a str, &'a str)>,
}

/// The truth's `gsd/E501` judgement when it is stale.
fn stale_verdict(truth: &PassEntity) -> Option<Verdict<'_>> {
    let judgement = truth
        .judgements
        .iter()
        .find(|j| j.rule == RULE && j.state == "stale")?;
    Some(Verdict {
        changed: &judgement.changed,
        files: judgement
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.digest.as_str()))
            .collect(),
    })
}

/// The files of the truth's `gsd/E501` judgement at the base, when the host gave them: a fresh or a stale judgement
/// (the states whose files it fills).
fn files_at_base(truth: &PassBaselineEntity) -> Option<Vec<(&str, &str)>> {
    let judgement = truth
        .judgements
        .iter()
        .find(|j| j.rule == RULE && matches!(j.state.as_str(), "fresh" | "stale"))?;
    Some(
        judgement
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.digest.as_str()))
            .collect(),
    )
}

/// How a message names the base: a revision by its first 12 characters, a session's base as its last check; `None`
/// for no base, where the pass says nothing.
fn base_name(base: &DeltaBase) -> Option<String> {
    match base {
        DeltaBase::None => None,
        DeltaBase::Revision { id } => Some(id.chars().take(12).collect()),
        DeltaBase::Previous => Some("the last check".to_string()),
    }
}

/// `(kind, id)` of what the truth's verdict reads, in [`READS`] order: the truth, its plan and its plan's phase, each
/// when it resolves.
fn read_by<'a>(
    input: &'a PassInput,
    truth: &'a PassEntity,
    plan: Option<&'a str>,
) -> Vec<(&'static str, &'a str)> {
    let phase = plan.and_then(|plan| phase_of(input, plan));
    [Some(truth.id.as_str()), plan, phase]
        .into_iter()
        .zip(READS)
        .filter_map(|(id, kind)| id.map(|id| (kind, id)))
        .collect()
}

/// Each of `read` the delta holds as added, or modified with a content digest that moved (a verdict recorded since
/// the base leaves it as it was), named `<kind> '<id>' (<what>)`, in the order of `read`.
fn spec_changes(delta: &DeltaPayload, read: &[(&str, &str)]) -> Vec<String> {
    read.iter()
        .filter_map(|(kind, id)| {
            let node = delta
                .nodes
                .iter()
                .find(|n| n.kind == *kind && n.id == *id)?;
            let what = match node.change {
                DeltaChange::Added => vec!["added".to_string()],
                DeltaChange::Modified if node.before != node.after => node
                    .fields
                    .iter()
                    .cloned()
                    .chain(
                        node.aspects
                            .iter()
                            .filter(|a| **a != DeltaAspect::Verdicts)
                            .map(aspect_name),
                    )
                    .collect(),
                DeltaChange::Modified | DeltaChange::Removed => return None,
            };
            Some(if what.is_empty() {
                format!("{kind} '{id}'")
            } else {
                format!("{kind} '{id}' ({})", what.join(", "))
            })
        })
        .collect()
}

/// An aspect's wire name (`title`, `obligations`).
fn aspect_name(aspect: &DeltaAspect) -> String {
    match serde_json::to_value(aspect) {
        Ok(serde_json::Value::String(name)) => name,
        other => unreachable!("an aspect serializes as its name, not {other:?}"),
    }
}

/// The files the truth's verdict cites whose digest at the base differs from now, in the current judgement's order;
/// `None` (unknown) unless the base is a revision the baseline was read at and the truth's judgement there lists its
/// files ([`files_at_base`]). A path the judgement at the base does not list is not named.
fn changed_files<'a>(
    input: &PassInput,
    delta: &DeltaPayload,
    truth: &PassEntity,
    now: &Verdict<'a>,
) -> Option<Vec<&'a str>> {
    let DeltaBase::Revision { id } = &delta.base else {
        return None;
    };
    let previous = input.previous.as_ref().filter(|p| p.revision == *id)?;
    let then = files_at_base(previous.entities.get(&truth.id)?)?;
    Some(
        now.files
            .iter()
            .filter(|(path, digest)| then.iter().any(|(p, d)| p == path && d != digest))
            .map(|(path, _)| *path)
            .collect(),
    )
}

/// The advisory: the verdict, the inputs it says changed since it was recorded, then what changed since the base.
fn message(
    truth: &str,
    verdict: &Verdict,
    base: &str,
    spec: &[String],
    files: Option<&[&str]>,
) -> String {
    let changed = if verdict.changed.is_empty() {
        String::new()
    } else {
        format!(
            " ({} changed since it was recorded)",
            verdict.changed.join(", ")
        )
    };
    let cites = quoted(verdict.files.iter().map(|(path, _)| *path));
    let it_cites = if cites.is_empty() {
        String::new()
    } else {
        format!("; it cites {cites}")
    };
    let cause = match (spec.is_empty(), files) {
        (false, Some([])) => format!(
            "the spec it reads changed: {}; no file it cites changed",
            spec.join(", ")
        ),
        (false, Some(files)) => format!(
            "the spec it reads changed: {}; the files it cites changed too: {}",
            spec.join(", "),
            quoted(files.iter().copied())
        ),
        (false, None) => format!("the spec it reads changed: {}{it_cites}", spec.join(", ")),
        (true, Some([])) => "neither the spec it reads nor a file it cites changed".to_string(),
        (true, Some(files)) => format!(
            "only the files it cites changed: {}",
            quoted(files.iter().copied())
        ),
        (true, None) => format!("the spec it reads is unchanged{it_cites}"),
    };
    format!("the verifier's verdict on truth '{truth}' is stale{changed}: since {base}, {cause}")
}

/// Each path in single quotes, escaped so a message stays one line, joined by `, `.
fn quoted<'a>(paths: impl Iterator<Item = &'a str>) -> String {
    paths
        .map(|path| format!("'{}'", path.escape_debug()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The command that lists the plan's verdicts that are not fresh, and why to run it.
fn suggestion(plan: &str) -> String {
    format!(
        "`specforge gsd re-verify {plan} --stale-only` lists the verdicts for the verifier to record again"
    )
}

#[cfg(test)]
mod tests;
