//! The computed values of verification (`spec/verify.md`, D-r39-7): a verification's status, score, time, covered files
//! and re-verification, and a verify gap's status.
//!
//! An item is `Verified` by a fresh passing `gsd/E501` verdict or an override (a truth), `Failed` by a failing verdict
//! or a scan finding (a file that is missing, short or lacks the text). The scan never verifies: that a file is there
//! does not show it is substantive and wired. Verdicts are recorded by `record_verdict` (r40); an item with no fresh
//! verdict is `Unknown` and keeps the verification from `passed`. A stale verdict is not a pass and not a failure.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{children, link, links, of_kind, phase_of};

type Statuses<'a> = BTreeMap<&'a str, &'static str>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Verified,
    Failed,
    Uncertain,
    BehaviorUnverified,
    Unknown,
}

/// A judgement as `(rule, state, passed, outcome)`.
type Row<'a> = (&'a str, &'a str, bool, Option<&'a str>);

/// The `gsd/E501` verdict on an entity: its outcome when it is fresh, else `Unknown`.
fn judged<'a>(mut rows: impl Iterator<Item = Row<'a>>) -> Item {
    let Some((_, _, passed, outcome)) = rows
        .find(|(rule, ..)| rule.ends_with("E501"))
        .filter(|(_, state, ..)| *state == "fresh")
    else {
        return Item::Unknown;
    };
    match outcome {
        Some("verified" | "verified_coincidental") if passed => Item::Verified,
        Some("failed") => Item::Failed,
        Some("uncertain") => Item::Uncertain,
        Some("present_behavior_unverified") => Item::BehaviorUnverified,
        _ => Item::Unknown,
    }
}

/// An item's state from what is known of it: its kind, the scan's `state`, its verdicts and whether a human overrode
/// it. The scan can fail an item, never verify one.
pub fn item_state(kind: &str, scan: Option<&str>, verdict: Item, overridden: bool) -> Item {
    if overridden {
        return Item::Verified;
    }
    let failed = matches!(
        (kind, scan),
        (
            "artifact",
            Some("missing" | "too_short" | "lacks_contains" | "lacks_export")
        ) | ("key_link", Some("not_wired"))
    );
    if failed { Item::Failed } else { verdict }
}

fn entity_item(e: &PassEntity, overridden: &BTreeSet<&str>) -> Item {
    item_state(
        &e.kind,
        e.fields.get("state").map(String::as_str),
        judged(e.judgements.iter().map(|j| {
            (
                j.rule.as_str(),
                j.state.as_str(),
                j.passed,
                j.outcome.as_deref(),
            )
        })),
        overridden.contains(e.id.as_str()),
    )
}

/// The status of a verification (D-r39-7), first match. A `partial` scan (a file of the phase the host did not
/// serve) cannot reach `passed`: it may have missed a blocker.
pub fn status(
    items: &[Item],
    truths: &[Item],
    blocker: bool,
    human_open: bool,
    partial: bool,
) -> &'static str {
    if blocker || items.contains(&Item::Failed) {
        "gaps_found"
    } else if human_open || truths.contains(&Item::BehaviorUnverified) {
        "human_needed"
    } else if !partial && !items.is_empty() && items.iter().all(|i| *i == Item::Verified) {
        "passed"
    } else {
        "pending"
    }
}

/// `{verified}/{truths}`: every truth that is not deferred counts in the denominator, a behavior-unverified one is
/// only left out of the numerator (`gsd-verifier.md`: `score: verified_truths / total_truths`).
pub fn score(truths: &[Item]) -> String {
    let verified = truths.iter().filter(|t| **t == Item::Verified).count();
    format!("{verified}/{}", truths.len())
}

struct Group<'a> {
    /// The phase's truths, artifacts and key links, minus the deferred truths.
    entities: Vec<&'a PassEntity>,
}

fn group<'a>(input: &'a PassInput, phase: &str, deferred: &BTreeSet<String>) -> Group<'a> {
    let mut entities = Vec::new();
    let mut plans: Vec<&PassEntity> = of_kind(input, "plan")
        .filter(|p| phase_of(input, &p.id) == Some(phase))
        .collect();
    plans.sort_by(|a, b| a.id.cmp(&b.id));
    for plan in plans {
        for kind in ["truth", "artifact", "key_link"] {
            entities.extend(
                children(input, &plan.id, kind)
                    .into_iter()
                    .filter(|e| !deferred.contains(&e.id)),
            );
        }
    }
    Group { entities }
}

fn deferred(v: &PassEntity) -> BTreeSet<String> {
    v.values
        .get("deferred")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("truth")?.as_str().map(String::from))
        .collect()
}

fn blocker(v: &PassEntity) -> bool {
    v.values
        .get("anti_patterns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|row| row.get("severity").and_then(Value::as_str) == Some("blocker"))
}

/// The scan could not read every file of the phase.
fn partial(v: &PassEntity) -> bool {
    v.values.get("scan_partial").and_then(Value::as_bool) == Some(true)
}

fn latest(group: &Group) -> Option<String> {
    group
        .entities
        .iter()
        .flat_map(|e| &e.judgements)
        .filter(|j| j.rule.ends_with("E501"))
        .filter_map(|j| j.at.clone())
        .max()
}

fn covered(group: &Group) -> Vec<String> {
    let files: BTreeSet<&str> = group
        .entities
        .iter()
        .flat_map(|e| &e.judgements)
        .filter(|j| j.rule.ends_with("E501"))
        .flat_map(|j| j.files.iter().map(|f| f.path.as_str()))
        .collect();
    files.into_iter().map(String::from).collect()
}

/// The state of an item at the baseline, from its verdicts only. The baseline's compute passes do get file facts, but
/// the git adapter loads only a commit's `*.spec` files and `specforge-harness`, so every source file reads as absent
/// there and the scan says `missing` (not `unchecked`) for a file the commit holds: a scan state computed at the
/// baseline says nothing about the commit, and neither does the status derived from it. Verdicts are judged at the
/// commit through an on-demand blob fetch, so they can be compared.
fn then_state(input: &PassInput, id: &str) -> Option<Item> {
    let e = input.previous.as_ref()?.entities.get(id)?;
    Some(judged(e.judgements.iter().map(|j| {
        (
            j.rule.as_str(),
            j.state.as_str(),
            j.passed,
            j.outcome.as_deref(),
        )
    })))
}

/// `re_verification`: the outcome against the baseline, over the items both views hold. An `Unknown` item on either
/// side is not compared. Items are the phase's truths, artifacts and key links (by verdict) and its verify gaps (by
/// status: open there, closed now). `previous_status` and `previous_score` are not built, for the reason above: they
/// wait for the kernel to serve a commit's source files to the baseline compile (K17), not for r40.
fn re_verification(
    input: &PassInput,
    v: &PassEntity,
    group: &Group,
    now: &[Item],
    statuses: &Statuses,
) -> Option<Value> {
    let previous = input.previous.as_ref()?;
    previous.entities.get(&v.id)?;
    let mut closed = Vec::new();
    let mut remaining = Vec::new();
    let mut regressions = Vec::new();
    for (e, state) in group.entities.iter().zip(now) {
        let Some(then) = then_state(input, &e.id) else {
            continue;
        };
        match (then, *state) {
            (Item::Failed, Item::Verified) => closed.push(e.id.clone()),
            (Item::Failed, Item::Unknown) => {}
            (Item::Failed, _) => remaining.push(e.id.clone()),
            (Item::Verified, Item::Failed) => regressions.push(e.id.clone()),
            _ => {}
        }
    }
    for gap in of_kind(input, "verify_gap")
        .filter(|g| link(input, g, "verification") == Some(v.id.as_str()))
    {
        let Some(was) = previous.entities.get(&gap.id) else {
            continue;
        };
        let was_open = was.fields.get("status").and_then(Value::as_str) == Some("open");
        let is_open = gap_closer(input, gap, statuses).is_none();
        match (was_open, is_open) {
            (true, false) => closed.push(gap.id.clone()),
            (true, true) => remaining.push(gap.id.clone()),
            (false, true) => regressions.push(gap.id.clone()),
            (false, false) => {}
        }
    }
    let mut block = serde_json::Map::new();
    for (name, ids) in [
        ("gaps_closed", closed),
        ("gaps_remaining", remaining),
        ("regressions", regressions),
    ] {
        if !ids.is_empty() {
            block.insert(name.into(), json!(ids));
        }
    }
    (!block.is_empty()).then_some(Value::Object(block))
}

/// What is known of one verification.
struct Assessment<'a> {
    verification: &'a PassEntity,
    group: Group<'a>,
    now: Vec<Item>,
    status: &'static str,
}

fn assess(input: &PassInput) -> Vec<Assessment<'_>> {
    let mut found = Vec::new();
    for v in of_kind(input, "verification") {
        let Some(phase) = link(input, v, "phase") else {
            continue;
        };
        let group = group(input, phase, &deferred(v));
        let overridden: BTreeSet<&str> = of_kind(input, "override")
            .filter(|o| link(input, o, "verification") == Some(v.id.as_str()))
            .filter_map(|o| link(input, o, "truth"))
            .collect();
        let now: Vec<Item> = group
            .entities
            .iter()
            .map(|e| entity_item(e, &overridden))
            .collect();
        let truths: Vec<Item> = group
            .entities
            .iter()
            .zip(&now)
            .filter(|(e, _)| e.kind == "truth")
            .map(|(_, i)| *i)
            .collect();
        let human_open = of_kind(input, "human_item")
            .filter(|h| link(input, h, "verification") == Some(v.id.as_str()))
            .any(|h| !passed_in_uat(input, h));
        let status = status(&now, &truths, blocker(v), human_open, partial(v));
        found.push(Assessment {
            verification: v,
            group,
            now,
            status,
        });
    }
    found
}

/// A phase's verification: its status and the time of the latest verdict.
pub struct Verified {
    pub status: &'static str,
    pub at: Option<String>,
}

/// The verification of each phase, by phase id.
pub fn by_phase(input: &PassInput) -> BTreeMap<String, Verified> {
    assess(input)
        .into_iter()
        .filter_map(|a| {
            let phase = link(input, a.verification, "phase")?.to_string();
            Some((
                phase,
                Verified {
                    status: a.status,
                    at: latest(&a.group),
                },
            ))
        })
        .collect()
}

pub fn values(input: &PassInput, statuses: &Statuses) -> Vec<PassValue> {
    let mut found = Vec::new();
    for a in assess(input) {
        let v = a.verification;
        let truths: Vec<Item> = a
            .group
            .entities
            .iter()
            .zip(&a.now)
            .filter(|(e, _)| e.kind == "truth")
            .map(|(_, i)| *i)
            .collect();
        found.push(value(&v.id, "status", a.status));
        found.push(value(&v.id, "score", score(&truths)));
        if let Some(at) = latest(&a.group) {
            found.push(value(&v.id, "verified", at));
        }
        let files = covered(&a.group);
        if !files.is_empty() {
            found.push(value(&v.id, "covered_files", json!(files)));
        }
        if let Some(block) = re_verification(input, v, &a.group, &a.now, statuses) {
            found.push(value(&v.id, "re_verification", block));
        }
    }
    found.extend(gaps(input, statuses));
    found
}

/// A human item is done once the UAT test it names passed.
fn passed_in_uat(input: &PassInput, item: &PassEntity) -> bool {
    link(input, item, "uat").is_some_and(|test| {
        of_kind(input, "uat_test")
            .any(|t| t.id == test && t.fields.get("result").map(String::as_str) == Some("pass"))
    })
}

/// The complete plan whose `gap_closure` names `gap`.
fn gap_closer<'a>(
    input: &'a PassInput,
    gap: &PassEntity,
    statuses: &Statuses,
) -> Option<&'a PassEntity> {
    of_kind(input, "plan")
        .filter(|p| links(input, p, "gap_closure").contains(&gap.id.as_str()))
        .find(|p| statuses.get(p.id.as_str()) == Some(&"complete"))
}

/// A verify gap is `closed` by a complete plan whose `gap_closure` names it.
fn gaps(input: &PassInput, statuses: &Statuses) -> Vec<PassValue> {
    of_kind(input, "verify_gap")
        .flat_map(|g| {
            let closer = gap_closer(input, g, statuses);
            let mut out = vec![value(
                &g.id,
                "status",
                if closer.is_some() { "closed" } else { "open" },
            )];
            out.extend(closer.map(|p| value(&g.id, "closed_by", p.id.clone())));
            out
        })
        .collect()
}

#[cfg(test)]
mod tests;
