//! The computed values of a risk (`spec/governance.md`): its `rpn` and `post_rpn` from its ratings, its `status` from
//! what its `mitigated_by` names, and whether it is `blocking` under `specforge.risk_block_on`.
//!
//! One seam rates a risk, [`rate`]: the product of severity, occurrence and detection, and of the post ratings, a post
//! member left out keeping its pre rating. A risk whose rating is missing or outside 1 to 10 gets no value: the kernel
//! reports it (a required field, E061), and no number is guessed.
//!
//! A mitigation is done when a task is done (a recorded commit names it, or its plan is complete: `execute::task_done`)
//! or a truth is verified (its phase's verification verifies it: `verify::items`). A name that does not resolve is
//! never done, and the prose `mitigation` is no evidence.
//!
//! gsd/W532 warns on a risk whose rpn is at or above the threshold and that names nothing in `mitigated_by`: this pass
//! holds the product, the threshold and the finding, so nothing reads a computed value back.
//!
//! A phase's `risk_level` is the highest current rpn of the risks that reach it (a task or a truth they name is of a
//! plan of the phase); a milestone's, the highest of its phases' and of the risks whose `milestone` names it.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::execute::{task_done, value};
use super::verify::{self, Item};
use crate::passes::commits::Commit;
use crate::passes::plans::{
    block, link, links, of_kind, phase_of, strings, targets, text, written_target,
};

/// A risk's three ratings, each 1 to 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sod {
    pub severity: i64,
    pub occurrence: i64,
    pub detection: i64,
}

impl Sod {
    fn product(self) -> i64 {
        self.severity * self.occurrence * self.detection
    }
}

/// The ratings of a risk's `post` block, each `None` when left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Post {
    pub severity: Option<i64>,
    pub occurrence: Option<i64>,
    pub detection: Option<i64>,
}

/// How far a risk's mitigations are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// It names no mitigation.
    Open,
    /// It names some, and one is not done.
    Mitigating,
    /// Every mitigation it names is done.
    Mitigated,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Mitigating => "mitigating",
            Status::Mitigated => "mitigated",
        }
    }
}

/// What a risk's ratings and mitigations compute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rating {
    /// severity × occurrence × detection.
    pub rpn: i64,
    /// The same product over the post ratings: `rpn` without a post block.
    pub post_rpn: i64,
    pub status: Status,
    /// The rpn the risk carries now: `post_rpn` once mitigated, `rpn` until then.
    pub current: i64,
    /// `current` is at or above the threshold.
    pub blocking: bool,
    /// It names no mitigation and its `rpn` is at or above the threshold.
    pub unmitigated: bool,
}

/// Rates a risk: pure and total over ratings already read in range. `done` holds one entry per written `mitigated_by`
/// item, `block_on` is `specforge.risk_block_on`.
pub fn rate(pre: Sod, post: Post, done: &[bool], block_on: i64) -> Rating {
    let after = Sod {
        severity: post.severity.unwrap_or(pre.severity),
        occurrence: post.occurrence.unwrap_or(pre.occurrence),
        detection: post.detection.unwrap_or(pre.detection),
    };
    let (rpn, post_rpn) = (pre.product(), after.product());
    let status = if done.is_empty() {
        Status::Open
    } else if done.iter().all(|d| *d) {
        Status::Mitigated
    } else {
        Status::Mitigating
    };
    let current = if status == Status::Mitigated {
        post_rpn
    } else {
        rpn
    };
    Rating {
        rpn,
        post_rpn,
        status,
        current,
        blocking: current >= block_on,
        unmitigated: status == Status::Open && rpn >= block_on,
    }
}

/// A rating as written: an integer from 1 to 10.
fn rating(written: &str) -> Option<i64> {
    written
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|r| (1..=10).contains(r))
}

/// A risk's ratings, `None` when a required one is missing or any written one is outside 1 to 10.
fn read(risk: &PassEntity) -> Option<(Sod, Post)> {
    let pre = |name: &str| text(risk, name).and_then(rating);
    let sod = Sod {
        severity: pre("severity")?,
        occurrence: pre("occurrence")?,
        detection: pre("detection")?,
    };
    let members = block(risk, "post");
    // `Some(None)`: left out; `None`: written and refused.
    let member = |name: &str| match members.get(name) {
        None => Some(None),
        Some(written) => rating(written).map(Some),
    };
    let post = Post {
        severity: member("severity")?,
        occurrence: member("occurrence")?,
        detection: member("detection")?,
    };
    Some((sod, post))
}

/// What the derive pass already knows of the things a risk may name: each read through its own kind, as an id is
/// unique only within its kind.
struct Done<'a> {
    commits: &'a [Commit],
    statuses: &'a BTreeMap<&'a str, &'static str>,
    tasks: BTreeMap<&'a str, &'a PassEntity>,
    items: BTreeMap<(&'a str, &'a str), Item>,
}

/// Whether the mitigation `id` (a resolved `mitigated_by` target) is done: a task done, or a truth verified.
fn mitigation_done(id: &str, done: &Done) -> bool {
    let task = done
        .tasks
        .get(id)
        .is_some_and(|task| task_done(task, done.commits, done.statuses));
    task || done.items.get(&("truth", id)) == Some(&Item::Verified)
}

/// One entry per item `risk` writes in `mitigated_by`: whether it is done; `false` for one that does not resolve. A
/// risk is unscoped, so an item names a target only as its full path, rooted or not: a short name that ends another
/// item's target is E003 and names nothing.
fn mitigations(input: &PassInput, risk: &PassEntity, done: &Done) -> Vec<bool> {
    let resolved = targets(input, &risk.id, "mitigated_by");
    strings(risk, "mitigated_by")
        .iter()
        .map(|written| {
            written_target(&resolved, &risk.id, written).is_some_and(|id| mitigation_done(id, done))
        })
        .collect()
}

/// Each rated risk with its rating.
fn rated<'a>(
    input: &'a PassInput,
    commits: &'a [Commit],
    statuses: &'a BTreeMap<&'a str, &'static str>,
    block_on: i64,
) -> Vec<(&'a PassEntity, Rating)> {
    let done = Done {
        commits,
        statuses,
        tasks: of_kind(input, "task").map(|t| (t.id.as_str(), t)).collect(),
        items: verify::items(input),
    };
    of_kind(input, "risk")
        .filter_map(|risk| {
            let (pre, post) = read(risk)?;
            Some((
                risk,
                rate(pre, post, &mitigations(input, risk, &done), block_on),
            ))
        })
        .collect()
}

/// gsd/W532 on `risk`, rated `rated` under `block_on`.
fn unmitigated(risk: &PassEntity, rated: &Rating, block_on: i64) -> PassDiagnostic {
    PassDiagnostic::new(
        "W532",
        PassSeverity::Warning,
        format!(
            "risk '{}' has rpn {}, at or above specforge.risk_block_on ({block_on}), and names nothing in mitigated_by",
            risk.id, rated.rpn
        ),
    )
    .with_suggestion("name the task or truth that controls it in mitigated_by, or rate it again")
    .with_entity(&risk.id)
}

/// Raises `levels[id]` to `current`.
fn raise<'a>(levels: &mut BTreeMap<&'a str, i64>, id: &'a str, current: i64) {
    let level = levels.entry(id).or_insert(current);
    *level = (*level).max(current);
}

/// `risk_level` of each phase a rated risk reaches and of each milestone that carries one. A mitigation's plan is read
/// kind by kind (a task's, a truth's `plan`), and a quick plan is in no phase.
fn risk_levels(input: &PassInput, rated: &[(&PassEntity, Rating)]) -> Vec<PassValue> {
    let plans: BTreeMap<&str, &str> = ["task", "truth"]
        .into_iter()
        .flat_map(|kind| of_kind(input, kind))
        .filter_map(|e| Some((e.id.as_str(), link(input, e, "plan")?)))
        .collect();
    let mut phases: BTreeMap<&str, i64> = BTreeMap::new();
    let mut milestones: BTreeMap<&str, i64> = BTreeMap::new();
    for (risk, rating) in rated {
        let reached: BTreeSet<&str> = links(input, risk, "mitigated_by")
            .into_iter()
            .filter_map(|target| plans.get(target))
            .filter_map(|plan| phase_of(input, plan))
            .collect();
        for phase in reached {
            raise(&mut phases, phase, rating.current);
        }
        if let Some(milestone) = link(input, risk, "milestone") {
            raise(&mut milestones, milestone, rating.current);
        }
    }
    let mut values = Vec::new();
    for phase in of_kind(input, "phase") {
        let Some(level) = phases.get(phase.id.as_str()).copied() else {
            continue;
        };
        values.push(value(&phase.id, "risk_level", level));
        if let Some(milestone) = link(input, phase, "milestone") {
            raise(&mut milestones, milestone, level);
        }
    }
    for milestone in of_kind(input, "milestone") {
        if let Some(level) = milestones.get(milestone.id.as_str()) {
            values.push(value(&milestone.id, "risk_level", *level));
        }
    }
    values
}

/// The values of each rated risk (`rpn`, `post_rpn`, `status`, `blocking`) and the `risk_level` of the phases and
/// milestones they reach, and gsd/W532 on each risk at or above the threshold that names no mitigation.
pub fn values_and_findings(
    input: &PassInput,
    commits: &[Commit],
    statuses: &BTreeMap<&str, &'static str>,
    block_on: i64,
) -> (Vec<PassValue>, Vec<PassDiagnostic>) {
    let risks = rated(input, commits, statuses, block_on);
    let mut values = Vec::new();
    let mut findings = Vec::new();
    for (risk, rated) in &risks {
        if rated.unmitigated {
            findings.push(unmitigated(risk, rated, block_on));
        }
        values.extend([
            value(&risk.id, "rpn", rated.rpn),
            value(&risk.id, "post_rpn", rated.post_rpn),
            value(&risk.id, "status", rated.status.as_str()),
            value(&risk.id, "blocking", rated.blocking),
        ]);
    }
    values.extend(risk_levels(input, &risks));
    (values, findings)
}

#[cfg(test)]
mod tests;
