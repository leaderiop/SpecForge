use serde_json::Value;
use specforge_ops::coverage::{CoverageQuery, CoverageRow};
use specforge_project::coverage::Status;

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.coverage`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Filter to specific entity
    entity_id: Option<String>,
    /// Filter by entity kind
    kind: Option<String>,
    /// Only entities with this coverage status
    #[arg(choice = specforge_ops::coverage::STATUS)]
    status_filter: Option<Status>,
}

/// `specforge.coverage`: the coverage view of the served project (its
/// recorded tests read at its root; with nothing served there is no root,
/// so none). With no filter, the entities that count toward
/// coverage, the ones stats counts as testable.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let query = CoverageQuery {
        entity_id: args.entity_id.as_deref(),
        kind: args.kind.as_deref(),
        status: args.status_filter,
    };
    match specforge_ops::coverage::coverage(&call.view(), &query) {
        Ok(outcome) => ToolOutcome::ok(Value::Array(
            outcome.rows.iter().map(CoverageRow::to_json).collect(),
        )),
        Err(error) => McpError::from(error).into(),
    }
}
