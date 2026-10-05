use serde_json::{Value, json};
use specforge_ops::coverage::{CoverageQuery, CoverageRow, parse_status};
use specforge_project::coverage::ReportError;

use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// A test report the tool cannot use, as an `McpError` (ADR 0004, D4-a):
/// `schema_mismatch` when it doesn't parse, `file_not_found` when a named
/// one doesn't exist, `internal_error` when it can't be read; the E045
/// diagnostic rides in `diagnostic`.
pub(crate) fn report_mcp_error(error: &ReportError, tool: &str) -> McpError {
    let code = match error {
        ReportError::Malformed { .. } => ErrorCode::SchemaMismatch,
        ReportError::Unreadable { missing: true, .. } => ErrorCode::FileNotFound,
        ReportError::Unreadable { .. } => ErrorCode::InternalError,
    };
    let mut mcp_error = McpError::new(code, error.to_string()).with_diagnostic(&error.diagnostic());
    mcp_error.tool = Some(tool.to_string());
    mcp_error
}

/// [`report_mcp_error`] as the tool's `isError` result.
pub(crate) fn report_error_result(error: &ReportError, tool: &str) -> ToolOutcome {
    report_mcp_error(error, tool).into()
}

/// One row of the coverage view as MCP spells it (`McpCoverageResult`):
/// the one presenter of a coverage row, for the coverage tool and every
/// view that lists rows.
pub(crate) fn row_json(row: &CoverageRow) -> Value {
    json!({
        "entity_id": row.entity_id,
        "kind": row.kind,
        "status": specforge_ops::coverage::status_name(row.status()),
        "declared": row.declared(),
        "linked": row.linked(),
        "evidence_collected": row.linked(),
        "obligations": row.verdict.obligations,
        "proven": row.verdict.proven,
        "unproven": row.verdict.unproven,
        "exempt": row.exempt,
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    entity_id: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    status_filter: Option<String>,
}

/// `specforge.coverage`: the coverage view of the served project (its
/// recorded tests read at its root; a graph built in memory with no
/// project has none). With no filter, the entities that count toward
/// coverage, the ones stats counts as testable.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let status = match args.status_filter.as_deref().map(parse_status).transpose() {
        Ok(status) => status,
        Err(error) => {
            return crate::operations::op_error(error)
                .with_argument("status_filter")
                .into();
        }
    };
    let query = CoverageQuery {
        entity_id: args.entity_id.as_deref(),
        kind: args.kind.as_deref(),
        status,
    };
    match specforge_ops::coverage::coverage(&call.view(), &query) {
        Ok(outcome) => ToolOutcome::ok(Value::Array(outcome.rows.iter().map(row_json).collect())),
        Err(error) => report_error_result(&error, "specforge.coverage"),
    }
}
