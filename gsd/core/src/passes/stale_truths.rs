//! The stale-truths pass: `gsd/I527`, one advisory per truth whose verifier verdict is stale, saying what changed since
//! the base the host names (ADR 0382). The stale set is `gsd/E501`'s judgements, never a digest of its own, so it
//! names exactly the truths `re_verify --stale-only` lists as stale. That command also lists a verdict that is
//! missing, unrecorded or unreadable; the pass names none of those, since such a verdict has no cited input that
//! changed since a base, and `gsd/E501` already reports each. What changed comes in two halves: the spec it reads,
//! from the graph delta of the truth, its plan and its phase; and the files it cites, from the truth's judgement at
//! the base and now, since a one-file edit is not in the delta (files are not graph nodes).
//!
//! With no base (a plain `check`, every node added) it says nothing: `gsd/E501` already reports the verdict. The files
//! half is exact only under a `revision` base whose baseline was read at the same commit; under a session's last check
//! it names the files the truth cites without claiming which changed, and so it does under a revision when none of the
//! files the verdict at the base read changed but it cites one now that it did not cite then.

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
    let mut stale: Vec<(&PassEntity, Option<&str>, Verdict)> = of_kind(input, "truth")
        .filter_map(|truth| {
            stale_verdict(truth).map(|verdict| (truth, link(input, truth, "plan"), verdict))
        })
        .collect();
    stale.sort_by(|a, b| reading_order(&a.0.id, a.1).cmp(&reading_order(&b.0.id, b.1)));
    stale
        .into_iter()
        .map(|(truth, plan, verdict)| {
            let read = read_by(input, truth, plan);
            let spec = spec_changes(delta, &read);
            let files = changed_files(input, delta, truth, &verdict);
            let message = message(&truth.id, &verdict, &base, &spec, files.as_ref());
            let found =
                PassDiagnostic::new("I527", PassSeverity::Info, message).with_entity(&truth.id);
            match plan {
                Some(plan) => found.with_suggestion(suggestion(plan)),
                None => found,
            }
        })
        .collect()
}

/// The order a person reads truths in: by their plan's [`id_order`] (`01-2` before `01-10`), then their own (`TR2`
/// before `TR10`). A truth's id ends in its own number, so [`id_order`] of the id alone puts `01-10/TR1` first.
fn reading_order<'a>(truth: &'a str, plan: Option<&'a str>) -> impl Ord + 'a {
    (plan.map(id_order), id_order(truth))
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
                    .map(|field| one_line(field))
                    .chain(
                        node.aspects
                            .iter()
                            .filter(|a| **a != DeltaAspect::Verdicts)
                            .map(|a| aspect_name(a).to_string()),
                    )
                    .collect(),
                DeltaChange::Modified | DeltaChange::Removed => return None,
            };
            let id = one_line(id);
            Some(if what.is_empty() {
                format!("{kind} '{id}'")
            } else {
                format!("{kind} '{id}' ({})", what.join(", "))
            })
        })
        .collect()
}

/// An aspect's wire name (`title`, `obligations`): a `match`, so a new aspect fails to compile until it is named.
fn aspect_name(aspect: &DeltaAspect) -> &'static str {
    match aspect {
        DeltaAspect::Title => "title",
        DeltaAspect::Methods => "methods",
        DeltaAspect::Obligations => "obligations",
        DeltaAspect::Verdicts => "verdicts",
        DeltaAspect::Edges => "edges",
    }
}

/// What the pass can say of the files a stale verdict cites, against the truth's judgement at the base.
struct Files<'a> {
    /// The files it cites whose digest at the base differs from now, in the current judgement's order.
    changed: Vec<&'a str>,
    /// The files it cites that the judgement at the base does not list: whether they changed is not known.
    uncited: Vec<&'a str>,
}

/// The files the truth's verdict cites, each changed since the base, unchanged or not cited there; `None` (unknown)
/// unless the base is a revision the baseline was read at and the truth's judgement there lists its files
/// ([`files_at_base`]). Also `None` when no file the judgement at the base lists changed but one it does not list is
/// cited now: such a file may have changed, so the pass cannot say that none did.
fn changed_files<'a>(
    input: &PassInput,
    delta: &DeltaPayload,
    truth: &PassEntity,
    now: &Verdict<'a>,
) -> Option<Files<'a>> {
    let DeltaBase::Revision { id } = &delta.base else {
        return None;
    };
    let previous = input.previous.as_ref().filter(|p| p.revision == *id)?;
    let then = files_at_base(previous.entities.get(&truth.id)?)?;
    let mut files = Files {
        changed: Vec::new(),
        uncited: Vec::new(),
    };
    for (path, digest) in &now.files {
        match then.iter().find(|(p, _)| p == path) {
            Some((_, before)) if before != digest => files.changed.push(*path),
            Some(_) => {}
            None => files.uncited.push(*path),
        }
    }
    (files.uncited.is_empty() || !files.changed.is_empty()).then_some(files)
}

/// The advisory: the verdict, the inputs it says changed since it was recorded, then what changed since the base.
fn message(
    truth: &str,
    verdict: &Verdict,
    base: &str,
    spec: &[String],
    files: Option<&Files>,
) -> String {
    let changed = if verdict.changed.is_empty() {
        String::new()
    } else {
        let inputs: Vec<String> = verdict.changed.iter().map(|name| one_line(name)).collect();
        format!(" ({} changed since it was recorded)", inputs.join(", "))
    };
    let cites = quoted(verdict.files.iter().map(|(path, _)| *path));
    let it_cites = if cites.is_empty() {
        String::new()
    } else {
        format!("; it cites {cites}")
    };
    let cause = match (spec.is_empty(), files.map(|f| f.changed.as_slice())) {
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
    let uncited = match files {
        Some(files) if !files.uncited.is_empty() => format!(
            "; it did not cite {} at the base",
            quoted(files.uncited.iter().copied())
        ),
        _ => String::new(),
    };
    let truth = one_line(truth);
    format!(
        "the verifier's verdict on truth '{truth}' is stale{changed}: since {base}, {cause}{uncited}"
    )
}

/// Each path in single quotes, [`one_line`], joined by `, `.
fn quoted<'a>(paths: impl Iterator<Item = &'a str>) -> String {
    paths
        .map(|path| format!("'{}'", one_line(path)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A text the pass echoes but did not write (a path, an id, a field or an input name), escaped so the message stays
/// one line. An id or a name of the grammar comes out unchanged; the one-line message does not rest on that.
fn one_line(text: &str) -> String {
    text.escape_debug().to_string()
}

/// The command that lists the plan's verdicts that are not fresh, and why to run it.
fn suggestion(plan: &str) -> String {
    let plan = one_line(plan);
    format!(
        "`specforge gsd re-verify {plan} --stale-only` lists the verdicts for the verifier to record again"
    )
}

#[cfg(test)]
mod tests;
