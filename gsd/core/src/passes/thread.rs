//! The tests of an entity, read one way from `@code/core`'s test facts (kind `test`, its `status`) and the graph's
//! edges (ADR 0331 D5, amended): the test facts that name an entity (an edge `names`, from a `[<ns>:<id>]` tag in the
//! test's name), its proof's facts (a truth's `proof`), and the facts a check or a verdict names (a deliverable
//! check's `ref`, a verdict's `cited_tests`: a fact's id, or `[file#]name`, ADR 0304). No report is read.
//!
//! "No test" has one meaning: test facts are collected and none links a passed or failed one to the entity. With no
//! test fact at all the evidence is `NotRun`, and each caller maps it once.

use specforge_extension_sdk::prelude::*;

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

/// The kind of `@code/core`'s test facts, which a truth's `proof` names, and the fields a deliverable check's `ref`
/// is matched on.
const TEST: &str = "test";
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
