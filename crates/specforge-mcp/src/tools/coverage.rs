use serde_json::Value;
use specforge_emitter::analyze::TestReport;
use specforge_emitter::coverage::{CoverageRegistries, ProjectCoverage, ReportError, Status};

use crate::state::McpState;
use crate::tool::ToolOutcome;

/// The project's `specforge-report.json` (written by `specforge collect`),
/// if there is one. Tests link themselves to entities by annotation
/// (ADR 0002), so recorded results are the linkage. A report that is there
/// but unreadable is an error, as in the CLI (ADR 0004, D2-e).
pub(crate) fn recorded_report(state: &McpState) -> Result<Option<TestReport>, ReportError> {
    match &state.project_root {
        Some(root) => specforge_emitter::coverage::read_report(root),
        None => Ok(None),
    }
}

/// A test report the tool cannot use, as an `McpError` (ADR 0004, D4-a):
/// `schema_mismatch` when it doesn't parse, `file_not_found` when a named
/// one doesn't exist, `internal_error` when it can't be read; the E045
/// diagnostic rides in `diagnostic`.
pub(crate) fn report_mcp_error(error: &ReportError, tool: &str) -> Value {
    let code = match error {
        ReportError::Malformed { .. } => "schema_mismatch",
        ReportError::Unreadable { missing: true, .. } => "file_not_found",
        ReportError::Unreadable { .. } => "internal_error",
    };
    let diagnostic: Value = serde_json::from_str(&specforge_emitter::serialize_diagnostics(&[
        error.diagnostic(),
    ]))
    .map(|mut all: Value| all[0].take())
    .unwrap_or(Value::Null);
    serde_json::json!({
        "code": code,
        "message": error.to_string(),
        "tool": tool,
        "diagnostic": diagnostic,
    })
}

/// [`report_mcp_error`] as the tool's `isError` result.
pub(crate) fn report_error_result(error: &ReportError, tool: &str) -> ToolOutcome {
    ToolOutcome::failed_with(report_mcp_error(error, tool))
}

/// The project's coverage under the one rule `analyze coverage` applies
/// (`specforge-coverage`), so no MCP view re-derives it: an entity is
/// covered exactly when that rule holds it proven, and never while analyze
/// reports A015 or A014 for it.
pub(crate) fn project_coverage(
    state: &McpState,
    tool: &str,
) -> Result<ProjectCoverage, ToolOutcome> {
    let report = recorded_report(state).map_err(|e| report_error_result(&e, tool))?;
    Ok(ProjectCoverage::compute(
        &state.graph,
        coverage_registries(state),
        report.as_ref(),
    ))
}

/// The served project's registries, as the coverage rule reads them.
pub(crate) fn coverage_registries(state: &McpState) -> CoverageRegistries<'_> {
    CoverageRegistries {
        kinds: &state.kind_registry,
        fields: &state.field_registry,
        rules: &state.rules,
    }
}

/// A coverage status as the MCP results spell it.
pub(crate) fn status_name(status: Status) -> &'static str {
    match status {
        Status::Covered => "covered",
        Status::Partial => "partial",
        Status::Uncovered => "uncovered",
    }
}

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let coverage = match project_coverage(state, "specforge.coverage") {
        Ok(coverage) => coverage,
        Err(outcome) => return outcome,
    };
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());
    let status_filter = args.get("status_filter").and_then(|v| v.as_str());

    // Testability is the extensions' call (their kinds' manifests).
    let testable = specforge_emitter::coverage::testable_kinds(&state.kind_registry);
    let results: Vec<Value> = state
        .graph
        .nodes()
        .into_iter()
        .filter(|n| {
            if let Some(eid) = entity_filter {
                return n.id.raw == eid;
            }
            testable.contains(n.kind.raw.as_str())
                && kind_filter.is_none_or(|kind| n.kind.raw == kind)
        })
        .filter_map(|n| Some((n, coverage.verdict(n.id.raw.as_str())?)))
        .filter(|(_, verdict)| status_filter.is_none_or(|s| status_name(verdict.status()) == s))
        .map(|(n, verdict)| {
            serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "status": status_name(verdict.status()),
                "declared": verdict.obligations > 0,
                "linked": verdict.tests > 0,
                "evidence_collected": verdict.tests > 0,
                "obligations": verdict.obligations,
                "proven": verdict.proven,
                "unproven": verdict.unproven,
            })
        })
        .collect();

    ToolOutcome::ok(Value::Array(results))
}
