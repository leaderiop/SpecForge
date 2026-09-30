use serde_json::Value;
use specforge_emitter::analyze::{ReportedTest, TestReport};
use specforge_emitter::coverage::ReportError;
use specforge_graph::Node;

use crate::state::McpState;
use crate::tool::ToolOutcome;

/// How a report's recorded status names a passing test.
const PASSING_STATUS: &str = "pass";

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

/// How an entity's recorded tests cover its `verify` obligations: the
/// rule `analyze coverage` applies (A015, A014), so the two never disagree.
pub(crate) struct EntityCoverage<'a> {
    pub obligations: usize,
    /// Verify texts no passing test names, in declaration order.
    pub unproven: Vec<&'a str>,
    pub tests: usize,
    pub failing: usize,
}

impl<'a> EntityCoverage<'a> {
    pub fn of(node: &'a Node, report: Option<&'a TestReport>) -> Self {
        let texts: Vec<&str> = specforge_emitter::coverage::obligations(node)
            .iter()
            .map(|s| s.description.as_str())
            .collect();
        let tests: &[ReportedTest] = report
            .and_then(|r| r.results.get(node.id.raw.as_str()))
            .map_or(&[], |e| e.tests.as_slice());
        let passing = |text: &str| {
            tests
                .iter()
                .any(|t| t.status == PASSING_STATUS && t.verify.as_deref() == Some(text))
        };
        EntityCoverage {
            obligations: texts.len(),
            unproven: texts.into_iter().filter(|t| !passing(t)).collect(),
            tests: tests.len(),
            failing: tests.iter().filter(|t| t.status != PASSING_STATUS).count(),
        }
    }

    pub fn proven(&self) -> usize {
        self.obligations - self.unproven.len()
    }

    /// `covered` when every obligation is proven and no test fails;
    /// `partial` when some obligation is proven or a test fails;
    /// `uncovered` otherwise, including an entity with no obligations.
    pub fn status(&self) -> &'static str {
        if self.obligations > 0 && self.unproven.is_empty() && self.failing == 0 {
            "covered"
        } else if self.proven() > 0 || self.failing > 0 {
            "partial"
        } else {
            "uncovered"
        }
    }
}

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let report = match recorded_report(state) {
        Ok(report) => report,
        Err(e) => return report_error_result(&e, "specforge.coverage"),
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
        .map(|n| (n, EntityCoverage::of(n, report.as_ref())))
        .filter(|(_, coverage)| status_filter.is_none_or(|s| coverage.status() == s))
        .map(|(n, coverage)| {
            serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "status": coverage.status(),
                "declared": coverage.obligations > 0,
                "linked": coverage.tests > 0,
                "evidence_collected": coverage.tests > 0,
                "obligations": coverage.obligations,
                "proven": coverage.proven(),
                "unproven": coverage.unproven,
            })
        })
        .collect();

    ToolOutcome::ok(Value::Array(results))
}
