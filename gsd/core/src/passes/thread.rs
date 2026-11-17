//! The tests of an entity, read one way from `@code/core`'s test facts (kind `test`, its `status`) and the graph's
//! edges (ADR 0331 D5, amended): the test facts that name an entity (an edge `names`, from a `[<ns>:<id>]` tag in the
//! test's name), its proof's facts (a truth's `proof`), and the facts a check or a verdict names (a deliverable
//! check's `ref`, a verdict's `cited_tests`: a fact's id, or `[file#]name`, ADR 0304). No report is read.
//!
//! "No test" has one meaning: test facts are collected and none links a passed or failed one to the entity. With no
//! test fact at all the evidence is `NotRun`, and each caller maps it once.
//!
//! A requirement is read one of two ways (ADR 0371 D5). When a plan lists it, its thread runs plan → task → commit →
//! test: the tasks of the plans that list it, the commits that name those tasks, and the tests linked to a committed
//! task, to the requirement, named in the `proof` of a truth that names it, or named by a check of a summary's
//! coverage item (a `deliverable`) whose `requirement` is it. When no plan lists it, it has no task to look for and
//! reads its tests alone: those that name it and those its coverage items' checks name. [`requirement_proof`] is the
//! one reading: the derive pass writes it as the requirement's `proof`, and `gsd/E515` reads that value through
//! [`requirement_proven`].

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits::Commit;
use super::plans::{of_kind, rows};

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

/// What the collected tests say of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// No test fact is collected.
    NotRun,
    /// Test facts are collected, and none of those given passed or failed.
    NoTest,
    /// One passed and none failed.
    Passed,
    /// One failed.
    Failed,
}

/// The kind of `@code/core`'s test facts, which a truth's `proof` names: every pass that reads the thread declares it
/// in its `facts` input (ADR 0353), and the fields a deliverable check's `ref` is matched on.
pub(crate) const TEST: &str = "test";
const NAME: &str = "name";
const FILE: &str = "file";
/// The field holding a test fact's result: the `result` `@code/core` declares for `test`. A pass input carries the
/// entities, not their kinds' declarations (protocol 1.15), so the pass cannot read it from the registry; it names the
/// peer's declared field once, here.
const STATUS: &str = "status";
/// A truth's fields the thread reads (`kinds/must_haves.rs`): the requirement it proves and the tests it cites.
const TRUTH: &str = "truth";
const REQUIREMENT: &str = "requirement";
const PROOF: &str = "proof";
/// The label of the edge from a test fact to each entity its `names` holds (`@code/core`).
const NAMES: &str = "names";
/// A summary's coverage item (`kinds/execute.rs`): the one kind whose `requirement` brings checks to a requirement's
/// proof. A `validation_row`, a `manual_check` and a `spec_prohibition` also write a `requirement`, and are not read.
const DELIVERABLE: &str = "deliverable";
/// A coverage item's checks, and the member of a check naming the test it runs.
const CHECKS: &str = "checks";
const REF: &str = "ref";
/// The kind whose `requirements` lists a requirement for its thread: a plan (a phase's `requirements` does not count).
const PLAN: &str = "plan";
const REQUIREMENTS: &str = "requirements";

/// Whether `reference` names the test fact `fact`: it is the fact's id, or `[file#]name`, where the first `#` ends
/// the file (a test name may hold one) and the fact's `name` is `name` and, when `file` is not empty, its `file` is
/// `file`. [`Thread::cited`] reads a reference that names no fact so as a whole name.
fn names_fact(fact: &PassEntity, reference: &str) -> bool {
    if fact.id == reference {
        return true;
    }
    let (file, test) = match reference.split_once('#') {
        Some((file, test)) => (Some(file).filter(|f| !f.is_empty()), test),
        None => (None, reference),
    };
    fact.fields.get(NAME).is_some_and(|n| n == test)
        && file.is_none_or(|file| fact.fields.get(FILE).is_some_and(|f| f == file))
}

/// The status a test fact's last run recorded; `None` for an entity that is no test fact, or holds no result.
fn status(fact: &PassEntity) -> Option<TestStatus> {
    let text = fact.fields.get(STATUS)?;
    TestStatus::ALL.into_iter().find(|s| s.as_str() == text)
}

/// The test facts of one pass input.
pub struct Thread<'a> {
    input: &'a PassInput,
}

impl<'a> Thread<'a> {
    pub fn of(input: &'a PassInput) -> Self {
        Self { input }
    }

    /// The test facts the input holds, with their status.
    fn facts(&self) -> impl Iterator<Item = (&'a PassEntity, TestStatus)> {
        self.input
            .entities
            .iter()
            .filter(|e| e.kind == TEST)
            .filter_map(|e| Some((e, status(e)?)))
    }

    /// A test run is recorded: the input holds a test fact with a status.
    pub fn ran(&self) -> bool {
        self.facts().next().is_some()
    }

    /// `(fact id, status)` of each test fact linked to `id`: those that name it, then its proof's.
    pub fn linked(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        let mut linked = self.naming(id);
        linked.extend(self.proof(id));
        linked
    }

    /// `(fact id, status)` of each test fact whose `names` holds `id`.
    pub fn naming(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        self.facts_at_ends(|e| e.target == id && e.label == NAMES, |e| &e.source)
    }

    /// `(fact id, status)` of the test fact at the chosen end of each edge `edge` keeps.
    fn facts_at_ends(
        &self,
        edge: impl Fn(&PassEdge) -> bool,
        end: impl Fn(&'a PassEdge) -> &'a String,
    ) -> Vec<(&'a str, TestStatus)> {
        self.input
            .edges
            .iter()
            .filter(|e| edge(e))
            .filter_map(|e| {
                let (fact, status) = self.facts().find(|(f, _)| f.id == *end(e))?;
                Some((fact.id.as_str(), status))
            })
            .collect()
    }

    /// `(fact id, status)` of each test fact `id`'s `proof` names (a truth's, ADR 0331 D2).
    pub fn proof(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        self.facts_at_ends(|e| e.source == id && e.label == PROOF, |e| &e.target)
    }

    /// `(fact id, status)` of each test fact the `proof` of a truth whose `requirement` is `requirement` names: what
    /// proves a requirement besides the facts linked to it and its tasks (ADR 0331 D1, D5).
    pub fn requirement_proofs(&self, requirement: &str) -> Vec<(&'a str, TestStatus)> {
        self.input
            .edges
            .iter()
            .filter(|e| e.label == REQUIREMENT && e.target == requirement)
            .filter(|e| {
                self.input
                    .entities
                    .iter()
                    .any(|t| t.kind == TRUTH && t.id == e.source)
            })
            .flat_map(|e| self.proof(&e.source))
            .collect()
    }

    /// The status of each test fact a check of a coverage item names, for every `deliverable` whose `requirement` is
    /// `requirement` (ADR 0371 D5): a check counts only through the collected fact its `ref` names, never by what the
    /// summary wrote, and a `ref` that names no fact gives nothing.
    pub fn coverage(&self, requirement: &str) -> Vec<TestStatus> {
        self.input
            .edges
            .iter()
            .filter(|e| e.label == REQUIREMENT && e.target == requirement)
            .filter_map(|e| {
                self.input
                    .entities
                    .iter()
                    .find(|d| d.kind == DELIVERABLE && d.id == e.source)
            })
            .flat_map(|deliverable| rows(deliverable, CHECKS))
            .filter_map(|check| check.get(REF).cloned())
            .flat_map(|reference| self.named(&reference))
            .collect()
    }

    /// The status of each test fact `reference` names (a deliverable check's `ref`, GSD's `test/x.test.js#name`): by
    /// the fact's id, or as `[file#]name` (the first `#` ends the file: a test name may hold one,
    /// `test/x.test.js#fixes #42`).
    pub fn named(&self, reference: &str) -> Vec<TestStatus> {
        self.cited(std::slice::from_ref(&reference))
    }

    /// The status of each test fact one of `texts` names, as [`Thread::named`] reads a reference (a verdict's
    /// `cited_tests`). A text that names no fact so is read whole as a test's name: `fixes #42` is a name, not the
    /// test `42` of a file `fixes `.
    pub fn cited(&self, texts: &[impl AsRef<str>]) -> Vec<TestStatus> {
        let named = |by: &dyn Fn(&PassEntity) -> bool| -> Vec<&'a str> {
            self.facts()
                .filter(|(fact, _)| by(fact))
                .map(|(fact, _)| fact.id.as_str())
                .collect()
        };
        let ids: Vec<&'a str> = texts
            .iter()
            .flat_map(|text| {
                let text = text.as_ref();
                let read = named(&|fact| names_fact(fact, text));
                if read.is_empty() {
                    named(&|fact| fact.fields.get(NAME).is_some_and(|n| n == text))
                } else {
                    read
                }
            })
            .collect();
        self.facts()
            .filter(|(fact, _)| ids.contains(&fact.id.as_str()))
            .map(|(_, status)| status)
            .collect()
    }

    /// `NotRun` unless a test fact is collected; then `Failed` if one of `statuses` failed, `Passed` if one passed, else
    /// `NoTest`.
    pub fn evidence(&self, statuses: impl IntoIterator<Item = TestStatus>) -> Evidence {
        if !self.ran() {
            return Evidence::NotRun;
        }
        let statuses: Vec<TestStatus> = statuses.into_iter().collect();
        if statuses.contains(&TestStatus::Failed) {
            Evidence::Failed
        } else if statuses.contains(&TestStatus::Passed) {
            Evidence::Passed
        } else {
            Evidence::NoTest
        }
    }

    /// The evidence of the test facts linked to `id`.
    pub fn of_entity(&self, id: &str) -> Evidence {
        self.evidence(self.linked(id).into_iter().map(|(_, status)| status))
    }
}

/// How far a requirement's thread reaches: for a requirement a plan lists, the first missing link of plan → task →
/// commit → test, then what its tests say; for one no plan lists, what its tests alone say (it never reads `NoTask`
/// or `NoCommit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proof {
    /// A plan lists it, and no plan that does has a task.
    NoTask,
    /// Its tasks have no recorded commit.
    NoCommit,
    /// No test fact is collected: a task is committed, or no plan lists it.
    NotRun,
    /// Test facts are collected, and none of its tests (those of a committed task, or, when no plan lists it, its own)
    /// passed or failed.
    NoTest,
    /// One of its tests failed, and, when a plan lists it, no committed task's tests passed.
    Failed,
    /// Its tests hold a passed one and no failed one (a plan lists it: those of one committed task).
    Proven,
}

impl Proof {
    pub const ALL: [Proof; 6] = [
        Proof::NoTask,
        Proof::NoCommit,
        Proof::NotRun,
        Proof::NoTest,
        Proof::Failed,
        Proof::Proven,
    ];

    /// Its word: the kind declares the words (`kinds::PROOF_STATES`), in the order of the variants.
    pub fn as_str(self) -> &'static str {
        crate::kinds::PROOF_STATES[self as usize]
    }

    /// The state a computed `proof` value names; `None` for any other text.
    pub fn parse(text: &str) -> Option<Proof> {
        Self::ALL.into_iter().find(|p| p.as_str() == text)
    }

    /// The first missing link, as `gsd/E515` names it; `None` when the thread is whole. A missing test is "no passing
    /// test" whichever reading gave it: a requirement no plan lists has no tasks to name.
    pub fn missing(self) -> Option<&'static str> {
        match self {
            Proof::NoTask => Some("no task of its plans"),
            Proof::NoCommit => Some("no commit of its tasks"),
            Proof::NotRun | Proof::NoTest | Proof::Failed => Some("no passing test"),
            Proof::Proven => None,
        }
    }
}

/// Every task, of any type, whose `plan` resolves to a plan that lists `requirement` in its `requirements`: the
/// route's `<plan.requirements` then `<task.plan`. A task's `plan` resolves to a plan only, so the sources of the
/// `requirements` edges need no kind check: a phase that lists the requirement is no task's plan.
pub fn requirement_tasks<'a>(input: &'a PassInput, requirement: &str) -> Vec<&'a PassEntity> {
    let listing: BTreeSet<&str> = input
        .edges
        .iter()
        .filter(|e| e.label == REQUIREMENTS && e.target == requirement)
        .map(|e| e.source.as_str())
        .collect();
    let tasks: BTreeSet<&str> = input
        .edges
        .iter()
        .filter(|e| e.label == "plan" && listing.contains(e.target.as_str()))
        .map(|e| e.source.as_str())
        .collect();
    of_kind(input, "task")
        .filter(|t| tasks.contains(t.id.as_str()))
        .collect()
}

/// Whether a plan lists `requirement` in its `requirements`: an edge `requirements` to it from an entity of kind
/// `plan`. A phase's `requirements` edge does not count, unlike in [`requirement_tasks`], where a task's `plan` already
/// keeps the plans only.
fn plan_lists(input: &PassInput, requirement: &str) -> bool {
    input
        .edges
        .iter()
        .filter(|e| e.label == REQUIREMENTS && e.target == requirement)
        .any(|e| of_kind(input, PLAN).any(|plan| plan.id == e.source))
}

/// The requirement's thread state (ADR 0331 D5, ADR 0371 D5). A requirement no plan lists reads its tests alone: with
/// test facts collected (else `NotRun`), the facts that name it and those its coverage items' checks name, `Proven`
/// when one passed and none failed, `Failed` when one failed, else `NoTest`. A requirement a plan lists reads its
/// thread: its tasks (none: `NoTask`), those a recorded commit names (none: `NoCommit`), then, with test facts
/// collected (else `NotRun`), the evidence of each committed task: the facts linked to it, those linked to the
/// requirement, the proofs of the truths that name the requirement, and those its coverage items' checks name. One
/// committed task whose evidence passed makes it `Proven`; else a failed one `Failed`; else `NoTest`.
pub fn requirement_proof(input: &PassInput, commits: &[Commit], requirement: &str) -> Proof {
    if !plan_lists(input, requirement) {
        let thread = Thread::of(input);
        let own = thread.naming(requirement).into_iter().map(|(_, s)| s);
        return match thread.evidence(own.chain(thread.coverage(requirement))) {
            Evidence::NotRun => Proof::NotRun,
            Evidence::NoTest => Proof::NoTest,
            Evidence::Passed => Proof::Proven,
            Evidence::Failed => Proof::Failed,
        };
    }
    let tasks = requirement_tasks(input, requirement);
    if tasks.is_empty() {
        return Proof::NoTask;
    }
    let committed: Vec<&PassEntity> = tasks
        .into_iter()
        .filter(|t| {
            commits
                .iter()
                .any(|c| c.task.as_deref() == Some(t.id.as_str()))
        })
        .collect();
    if committed.is_empty() {
        return Proof::NoCommit;
    }
    let thread = Thread::of(input);
    if !thread.ran() {
        return Proof::NotRun;
    }
    let shared: Vec<TestStatus> = thread
        .linked(requirement)
        .into_iter()
        .chain(thread.requirement_proofs(requirement))
        .map(|(_, status)| status)
        .chain(thread.coverage(requirement))
        .collect();
    let evidence: Vec<Evidence> = committed
        .iter()
        .map(|task| {
            let linked = thread.linked(&task.id).into_iter().map(|(_, s)| s);
            thread.evidence(linked.chain(shared.iter().copied()))
        })
        .collect();
    if evidence.contains(&Evidence::Passed) {
        Proof::Proven
    } else if evidence.contains(&Evidence::Failed) {
        Proof::Failed
    } else {
        Proof::NoTest
    }
}

/// Whether a requirement is proven: it is `complete` and its thread is whole. `gsd/E515` refuses a complete
/// requirement for which it is false.
pub fn requirement_proven(status: Option<&str>, proof: Proof) -> bool {
    status == Some("complete") && proof == Proof::Proven
}
