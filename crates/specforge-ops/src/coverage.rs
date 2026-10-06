//! The coverage view: each entity's coverage under the one rule
//! `analyze coverage` applies, over the project view (ADR 0015).
//!
//! The MCP coverage tool, inspect, query and the review prompt read it;
//! with no entity named it lists exactly the entities stats counts as
//! testable, so its rows and stats' numbers cannot disagree.

use specforge_project::coverage::{ReportError, Status, Summary, Verdict};
use specforge_project::snapshot::{EntityRecord, Standing};

use crate::OpError;
use crate::view::ProjectView;

/// Which rows to list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoverageQuery<'q> {
    /// That entity alone, whether it counts toward coverage or not.
    pub entity_id: Option<&'q str>,
    /// Without `entity_id`: only entities of this kind.
    pub kind: Option<&'q str>,
    /// Only rows with this status.
    pub status: Option<Status>,
}

/// One entity's coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRow {
    pub entity_id: String,
    pub kind: String,
    /// Its kind is testable.
    pub testable: bool,
    /// A testable-kind entity that owes no obligations and declares none
    /// (W004 exempts it): it does not count toward coverage
    /// (`Standing::exempt`).
    pub exempt: bool,
    pub verdict: Verdict,
}

impl CoverageRow {
    fn of(record: &EntityRecord, standing: &Standing, verdict: &Verdict) -> Self {
        CoverageRow {
            entity_id: record.id.clone(),
            kind: record.kind.clone(),
            testable: standing.testable,
            exempt: standing.exempt(),
            verdict: verdict.clone(),
        }
    }

    pub fn status(&self) -> Status {
        self.verdict.status()
    }

    /// It declares at least one obligation.
    pub fn declared(&self) -> bool {
        self.verdict.obligations > 0
    }

    /// Tests are recorded for it.
    pub fn linked(&self) -> bool {
        self.verdict.tests > 0
    }

    /// It counts toward coverage and is not proven
    /// (`ProjectCoverage::is_unverified`).
    pub fn unverified(&self) -> bool {
        self.testable && !self.exempt && !self.verdict.is_proven()
    }
}

/// The rows a query selects, and the project's coverage summary.
#[derive(Debug, Clone)]
pub struct CoverageOutcome {
    pub rows: Vec<CoverageRow>,
    pub summary: Summary,
}

/// The coverage view. With no `entity_id`: the entities that count toward
/// coverage, so `rows.len() == summary.testable_total`, narrowed by kind
/// and status, in entity id order. With `entity_id`: that entity, counted
/// or not (an exempt row says so), or no row when the graph lacks it. A
/// recorded report that cannot be read is the error.
pub fn coverage(view: &ProjectView, query: &CoverageQuery) -> Result<CoverageOutcome, ReportError> {
    let coverage = view.coverage()?;
    let rows = coverage
        .entities()
        .iter()
        .filter(|(record, standing)| match query.entity_id {
            Some(entity_id) => record.id == entity_id,
            None => standing.counts() && query.kind.is_none_or(|kind| record.kind == kind),
        })
        .filter_map(|(record, standing)| {
            Some(CoverageRow::of(
                record,
                standing,
                coverage.verdict(&record.id)?,
            ))
        })
        .filter(|row| query.status.is_none_or(|status| row.status() == status))
        .collect();
    Ok(CoverageOutcome {
        rows,
        summary: coverage.summary.clone(),
    })
}

/// One entity's row, counted or not; `None` when the graph lacks it.
pub fn row(view: &ProjectView, entity_id: &str) -> Result<Option<CoverageRow>, ReportError> {
    let coverage = view.coverage()?;
    Ok(coverage
        .entities()
        .get(entity_id)
        .zip(coverage.verdict(entity_id))
        .map(|((record, standing), verdict)| CoverageRow::of(record, standing, verdict)))
}

/// Every status, in the order a listing names them.
const STATUSES: [Status; 3] = [Status::Covered, Status::Partial, Status::Uncovered];

/// A coverage status as the results spell it (`covered`, `partial`,
/// `uncovered`), as it serializes.
pub fn status_name(status: Status) -> &'static str {
    match status {
        Status::Covered => "covered",
        Status::Partial => "partial",
        Status::Uncovered => "uncovered",
    }
}

/// A coverage status by name ([`status_name`]); any other is
/// `invalid_input`, naming the closest one.
pub fn parse_status(name: &str) -> Result<Status, OpError> {
    if let Some(status) = STATUSES.into_iter().find(|s| status_name(*s) == name) {
        return Ok(status);
    }
    let names = STATUSES.map(status_name);
    let error = OpError::new(
        "invalid_input",
        format!(
            "unknown coverage status '{name}' (expected one of: {})",
            names.join(", ")
        ),
    );
    Err(
        match specforge_common::suggest::find_close_match(name, names) {
            Some(close) => error.with_suggestion(format!("did you mean '{close}'?")),
            None => error,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_names_parse_and_others_are_refused_with_the_closest() {
        for status in STATUSES {
            assert_eq!(parse_status(status_name(status)), Ok(status));
            assert_eq!(
                serde_json::to_value(status).unwrap(),
                status_name(status),
                "the name is the serialized form"
            );
        }
        let error = parse_status("coverd").unwrap_err();
        assert_eq!(error.code, "invalid_input");
        assert_eq!(error.suggestion.as_deref(), Some("did you mean 'covered'?"));
    }
}
