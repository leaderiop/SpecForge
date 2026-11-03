//! The tests of an entity, read one way (ADR 0331 D5): the results recorded for it (`[gsd:<id>]` tags, ADR 0312),
//! the results its verdict cites by name (ADR 0304) and, for a truth, the results of its proof's test facts
//! (`@code/core`'s `test`, its `status`).
//!
//! "No test" has one meaning: a recorded run that links no passed or failed result to the entity. With no run at all
//! (no report, or a report with no result in it, what the git collector writes, and no test fact) the evidence is
//! `NotRun`, and each caller maps it once.

use specforge_extension_sdk::prelude::*;

#[cfg(test)]
mod tests;

/// What the recorded tests say of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// No test run is recorded.
    NotRun,
    /// A run is recorded, and none of the results given passed or failed.
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

/// The status a test fact's last run recorded; `None` for an entity that is no test fact, or holds no result.
fn status(fact: &PassEntity) -> Option<TestStatus> {
    let text = fact.fields.get(STATUS)?;
    TestStatus::ALL.into_iter().find(|s| s.as_str() == text)
}

/// The recorded test results of one pass input.
pub struct Thread<'a> {
    input: &'a PassInput,
}

impl<'a> Thread<'a> {
    pub fn of(input: &'a PassInput) -> Self {
        Self { input }
    }

    fn results(&self) -> impl Iterator<Item = &'a TestResult> {
        self.input
            .test_results
            .iter()
            .flat_map(|report| report.results.values())
            .flat_map(|recorded| &recorded.tests)
    }

    /// The test facts the input holds, with their status.
    fn facts(&self) -> impl Iterator<Item = (&'a PassEntity, TestStatus)> {
        self.input
            .entities
            .iter()
            .filter(|e| e.kind == TEST)
            .filter_map(|e| Some((e, status(e)?)))
    }

    /// A test run is recorded: the report holds at least one result, or the input holds a test fact.
    pub fn ran(&self) -> bool {
        self.results().next().is_some() || self.facts().next().is_some()
    }

    /// `(name, status)` of each result linked to `id`: its tagged results, then its proof's facts by id.
    pub fn linked(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        let mut linked: Vec<(&'a str, TestStatus)> = self
            .input
            .test_results
            .as_ref()
            .map(|report| {
                report
                    .of(id)
                    .iter()
                    .map(|t| (t.name.as_str(), t.status))
                    .collect()
            })
            .unwrap_or_default();
        linked.extend(self.proof(id));
        linked
    }

    /// `(fact id, status)` of each test fact `id`'s `proof` names (a truth's, ADR 0331 D2).
    pub fn proof(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        self.input
            .edges
            .iter()
            .filter(|e| e.source == id && e.label == PROOF)
            .filter_map(|e| {
                let (fact, status) = self.facts().find(|(f, _)| f.id == e.target)?;
                Some((fact.id.as_str(), status))
            })
            .collect()
    }

    /// `(fact id, status)` of each test fact the `proof` of a truth whose `requirement` is `requirement` names: what
    /// proves a requirement besides the results linked to it and its tasks (ADR 0331 D1, D5).
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

    /// The results of `id` named `name` (a deliverable check's `ref`, GSD's `test/x.test.js#name`), and the status of
    /// each test fact it names: the fact's `name` is the ref's name and, when the ref names a file, its `file` is it.
    pub fn named(&self, id: &str, name: &str) -> Vec<TestStatus> {
        let (file, test) = match name.rsplit_once('#') {
            Some((file, test)) => (Some(file), test),
            None => (None, name),
        };
        let facts = self.facts().filter(|(fact, _)| {
            fact.fields.get(NAME).is_some_and(|n| n == test)
                && file.is_none_or(|file| fact.fields.get(FILE).is_some_and(|f| f == file))
        });
        self.linked(id)
            .into_iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, status)| status)
            .chain(facts.map(|(_, status)| status))
            .collect()
    }

    /// The results anywhere in the report whose name is one of `names` (a verdict's `cited_tests`).
    pub fn cited(&self, names: &[String]) -> Vec<TestStatus> {
        self.results()
            .filter(|t| names.contains(&t.name))
            .map(|t| t.status)
            .collect()
    }

    /// `NotRun` unless a run is recorded; then `Failed` if one of `statuses` failed, `Passed` if one passed, else
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

    /// The evidence of the results recorded for `id`.
    pub fn of_entity(&self, id: &str) -> Evidence {
        self.evidence(self.linked(id).into_iter().map(|(_, status)| status))
    }
}
