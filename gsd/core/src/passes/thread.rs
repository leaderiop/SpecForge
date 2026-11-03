//! The tests of an entity, read one way (ADR 0331 D5): the results recorded for it (`[gsd:<id>]` tags, ADR 0312) and
//! the results its verdict cites by name (ADR 0304).
//!
//! "No test" has one meaning: a recorded run that links no passed or failed result to the entity. With no run at all
//! (no report, or a report with no result in it: what the git collector writes) the evidence is `NotRun`, and each
//! caller maps it once.

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

    /// A test run is recorded: the report holds at least one result.
    pub fn ran(&self) -> bool {
        self.results().next().is_some()
    }

    /// `(name, status)` of each result recorded for `id`.
    pub fn linked(&self, id: &str) -> Vec<(&'a str, TestStatus)> {
        self.input
            .test_results
            .as_ref()
            .map(|report| {
                report
                    .of(id)
                    .iter()
                    .map(|t| (t.name.as_str(), t.status))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The results recorded for `id` named `name` (a deliverable check's `ref`).
    pub fn named(&self, id: &str, name: &str) -> Vec<TestStatus> {
        self.linked(id)
            .into_iter()
            .filter(|(n, _)| *n == name)
            .map(|(_, status)| status)
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
