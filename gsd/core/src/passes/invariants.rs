//! When an invariant is due (`spec/governance.md`, "When an invariant is due"): a phase owes an invariant a judgement
//! once one of its commits changed code the invariant guards after the invariant was last judged (`gsd/I536`).
//!
//! What an invariant guards: its `items`; the items its proof's tests and the tests naming it run as
//! (`@code/core`'s `runs_as` edge, from an item to the test it is the function of), whether or not those tests ran;
//! and the items whose name tags it. A commit whose `items_changed` is unwritten is matched by the files it touched,
//! against the files those items and tests are in.
//!
//! Only an invariant in force and judged can be due. In force is its computed `status` (the `derive` pass answers it
//! before this pass runs) in the kind's `complete_when`, the owner state `gsd/I536`'s `judged_while: complete` reads,
//! so a pending invariant owes nothing here as it owes no verdict there. Judged is an I536 verdict, fresh or stale,
//! whatever its outcome, with a time: it covers every commit authored at or before it (the second the host recorded
//! it). An invariant never judged owes nothing; I536 reports its missing verdict.
//!
//! [`due`] is the one join of recorded commits with guarded code. The `invariants` compute pass answers
//! `phase.invariants_due` from its rows and reports `gsd/E531` from the same rows, one error per invariant and phase. A reader of which items a test exercises (coverage) adds a source to what
//! an invariant guards, inside [`due`], and every reader of its rows follows unchanged.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::commits::{Commit, commits};
use super::derive::invariant::verdict;
use super::plans::{link, of_kind, phase_of, strings, text};
use super::roadmap::numbering::Number;
use super::thread::{NAMES, TEST, Thread};
use crate::kinds::INVARIANT_IN_FORCE;
use crate::scope::scope_of;
use crate::time::{seconds, verdict_time};

/// The kind of `@code/core`'s item facts: the code an invariant guards, and the files they are in.
const ITEM: &str = "item";
/// The label of the edge from an item fact to the test fact it runs as (its `test_id`, `@code/core`).
const RUNS_AS: &str = "runs_as";
const INVARIANT: &str = "invariant";

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("invariants", |p| {
        p.compute()
            .computes(&["phase.invariants_due"])
            .input(Input::facts(&[TEST, ITEM]))
            .run(compute);
    });
}

fn compute(input: &PassInput) -> PassOutput {
    let rows = due(input);
    PassOutput {
        values: invariants_due(&rows),
        diagnostics: e531(&rows),
        ..Default::default()
    }
}

/// `commit`, of `phase`, changed `item`, which `invariant` guards, after the invariant was last judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    pub invariant: String,
    pub phase: String,
    pub commit: String,
    /// The guarded item, or the guarded test (its id) when the row came from a file.
    pub item: String,
    /// The path the commit touched, when its `items_changed` is unwritten and `item` was matched by the file it is in;
    /// `None` when the commit's `items_changed` names `item`.
    pub file: Option<String>,
}

/// Every due row: by invariant id, then commit (oldest first, [`commits`]'s order), then item.
pub fn due(input: &PassInput) -> Vec<Due> {
    let recorded = commits(input);
    let code = Code::of(input);
    let mut invariants: Vec<&PassEntity> =
        of_kind(input, INVARIANT).filter(|x| in_force(x)).collect();
    invariants.sort_by(|a, b| a.id.cmp(&b.id));
    let mut rows = Vec::new();
    for invariant in invariants {
        let Some(judged) = judged_at(invariant) else {
            continue;
        };
        let guard = guarded(input, &code, invariant);
        let window = Window::of(input, invariant, &recorded);
        for commit in &recorded {
            let Some(phase) = commit.plan.as_deref().and_then(|p| phase_of(input, p)) else {
                continue;
            };
            let at = seconds(&commit.at);
            if !window.admits(phase, at) || at.is_some_and(|at| judged >= at) {
                continue;
            }
            let row = |item: &str, file: Option<&str>| Due {
                invariant: invariant.id.clone(),
                phase: phase.to_string(),
                commit: commit.id.clone(),
                item: item.to_string(),
                file: file.map(String::from),
            };
            match &commit.items_changed {
                Some(changed) => {
                    let hit: BTreeSet<&str> = changed
                        .iter()
                        .map(String::as_str)
                        .filter(|item| guard.items.contains(*item))
                        .collect();
                    rows.extend(hit.into_iter().map(|item| row(item, None)));
                }
                None => {
                    let mut hit: Vec<(&str, &str)> = commit
                        .files
                        .iter()
                        .filter_map(|path| {
                            Some((guard.files.get(path.as_str())?.as_str(), path.as_str()))
                        })
                        .collect();
                    hit.sort();
                    hit.dedup();
                    rows.extend(hit.into_iter().map(|(item, path)| row(item, Some(path))));
                }
            }
        }
    }
    rows
}

/// Whether `invariant` is in force: its computed `status` is one its kind's `complete_when` holds on.
fn in_force(invariant: &PassEntity) -> bool {
    text(invariant, "status").is_some_and(|status| INVARIANT_IN_FORCE.contains(&status))
}

/// Until when `invariant`'s last judgement covers a commit: the time of its `gsd/I536` verdict, fresh or stale,
/// whatever its outcome ([`verdict_time`]: the second the host recorded it); `None` with no verdict recorded
/// (missing), one the host cannot read (unrecorded, unreadable) or a time it cannot read.
fn judged_at(invariant: &PassEntity) -> Option<i64> {
    match verdict(invariant)? {
        ("fresh" | "stale", _, Some(at)) => verdict_time(at),
        _ => None,
    }
}

/// The code facts of one pass input, read once for every invariant: the ids of the item facts, and the file of each
/// item and test fact.
struct Code<'a> {
    items: BTreeSet<&'a str>,
    files: BTreeMap<&'a str, &'a str>,
}

impl<'a> Code<'a> {
    fn of(input: &'a PassInput) -> Self {
        let facts = || {
            input
                .entities
                .iter()
                .filter(|e| e.kind == ITEM || e.kind == TEST)
        };
        Code {
            items: facts()
                .filter(|e| e.kind == ITEM)
                .map(|e| e.id.as_str())
                .collect(),
            files: facts()
                .filter_map(|e| Some((e.id.as_str(), text(e, "file")?)))
                .collect(),
        }
    }
}

/// What an invariant guards: the ids of its code items, and each file one of them or one of its tests is in, with the
/// first guarded id (by id order) the file holds.
#[derive(Debug, Default, PartialEq, Eq)]
struct Guarded {
    items: BTreeSet<String>,
    files: BTreeMap<String, String>,
}

/// What `invariant` guards: its `items` (as written, as W522 reads `task.items`), the items its tests run as (its
/// tests, run or not: [`Thread::tests`]) and the items whose name tags it.
fn guarded(input: &PassInput, code: &Code<'_>, invariant: &PassEntity) -> Guarded {
    let tests = Thread::of(input).tests(&invariant.id);
    let mut items: BTreeSet<String> = strings(invariant, "items").into_iter().collect();
    items.extend(
        input
            .edges
            .iter()
            .filter(|e| match e.label.as_str() {
                RUNS_AS => tests.contains(e.target.as_str()),
                NAMES => e.target == invariant.id,
                _ => false,
            })
            .filter(|e| code.items.contains(e.source.as_str()))
            .map(|e| e.source.clone()),
    );
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    let ids = items
        .iter()
        .map(String::as_str)
        .chain(tests.iter().copied());
    for id in ids {
        if let Some(path) = code.files.get(id) {
            let first = files
                .entry(path.to_string())
                .or_insert_with(|| id.to_string());
            if id < first.as_str() {
                *first = id.to_string();
            }
        }
    }
    Guarded { items, files }
}

/// The phases whose commits can owe an invariant a judgement: every phase with no `holds_from`; else its `holds_from`
/// phase, a later-numbered phase of its milestone and, where numbers do not order two phases (another milestone, a
/// custom-named or backlog phase), a commit no older than the `holds_from` phase's first recorded commit. Under
/// `scope milestone`, only the `holds_from` phase's milestone.
enum Window<'a> {
    Every,
    From {
        phase: &'a str,
        number: Option<Number>,
        /// The time of the `holds_from` phase's first recorded commit; `None` while it has none.
        start: Option<i64>,
        /// `scope milestone`: no other milestone's commit counts.
        one_milestone: bool,
    },
}

/// A phase's number when the numbering orders it: a backlog `999.x` sits outside it.
fn ordered(phase: &str) -> Option<Number> {
    Number::of(phase).filter(|n| !n.backlog())
}

impl<'a> Window<'a> {
    fn of(input: &'a PassInput, invariant: &PassEntity, commits: &[Commit]) -> Self {
        let Some(phase) = link(input, invariant, "holds_from") else {
            return Window::Every;
        };
        let start = commits
            .iter()
            .filter(|c| c.plan.as_deref().and_then(|p| phase_of(input, p)) == Some(phase))
            .filter_map(|c| seconds(&c.at))
            .min();
        Window::From {
            phase,
            number: ordered(phase),
            start,
            one_milestone: text(invariant, "scope") == Some("milestone"),
        }
    }

    /// Whether a commit of `phase` authored at `at` (`None`: a time it cannot read) is in the window.
    fn admits(&self, phase: &str, at: Option<i64>) -> bool {
        let Window::From {
            phase: from,
            number,
            start,
            one_milestone,
        } = self
        else {
            return true;
        };
        if phase == *from {
            return true;
        }
        let by_time = || matches!((start, at), (Some(start), Some(at)) if at >= *start);
        if scope_of(phase) != scope_of(from) {
            return !one_milestone && by_time();
        }
        match (number, ordered(phase)) {
            (Some(from), Some(number)) => number > *from,
            _ => by_time(),
        }
    }
}

/// `phase.invariants_due`: per phase with a row, its due invariants once each, in id order.
fn invariants_due(rows: &[Due]) -> Vec<PassValue> {
    let mut by_phase: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for row in rows {
        by_phase
            .entry(&row.phase)
            .or_default()
            .insert(&row.invariant);
    }
    by_phase
        .into_iter()
        .map(|(phase, invariants)| PassValue {
            entity: phase.to_string(),
            field: "invariants_due".to_string(),
            value: invariants.into_iter().collect::<Vec<_>>().into(),
        })
        .collect()
}

/// `gsd/E531`: one Error per invariant and phase with a row, in row order, on the invariant. The message names the
/// invariant and the phase only, so a write that adds a row to a phase already due changes no error (E080 compares
/// messages); the suggestion names each commit, in order, and what it changed or touched.
fn e531(rows: &[Due]) -> Vec<PassDiagnostic> {
    let mut due: Vec<((&str, &str), Vec<&Due>)> = Vec::new();
    for row in rows {
        let key = (row.invariant.as_str(), row.phase.as_str());
        match due.iter_mut().find(|(k, _)| *k == key) {
            Some((_, found)) => found.push(row),
            None => due.push((key, vec![row])),
        }
    }
    due.into_iter()
        .map(|((invariant, phase), rows)| {
            let mut commits: Vec<&str> = Vec::new();
            for row in &rows {
                if !commits.contains(&row.commit.as_str()) {
                    commits.push(&row.commit);
                }
            }
            let what: Vec<String> = commits
                .into_iter()
                .flat_map(|commit| {
                    let of: Vec<&&Due> = rows.iter().filter(|r| r.commit == commit).collect();
                    let changed: Vec<&str> = of
                        .iter()
                        .filter(|r| r.file.is_none())
                        .map(|r| r.item.as_str())
                        .collect();
                    let mut said = Vec::new();
                    if !changed.is_empty() {
                        said.push(format!("commit {commit} changed {}", changed.join(", ")));
                    }
                    said.extend(of.iter().filter_map(|r| {
                        let path = r.file.as_deref()?;
                        Some(format!("commit {commit} touched {path} ({})", r.item))
                    }));
                    said
                })
                .collect();
            PassDiagnostic::new(
                "E531",
                PassSeverity::Error,
                format!(
                    "invariant '{invariant}' is due in phase '{phase}': the phase changed what it guards after it \
                     was last judged"
                ),
            )
            .with_suggestion(format!(
                "judge it again (verify_invariant --invariant {invariant} records its gsd/I536 verdict): {}",
                what.join("; ")
            ))
            .with_entity(invariant)
        })
        .collect()
}

#[cfg(test)]
mod tests;
