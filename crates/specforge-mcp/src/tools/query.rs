use serde_json::Value;
use specforge_ops::export::Format;
use specforge_ops::query::{QueryRequest, query};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.query`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to query
    entity_id: String,
    /// Number of hops
    #[arg(default = 1)]
    depth: usize,
    /// Filter by entity kinds
    kinds: Vec<String>,
    /// Output detail level
    #[arg(choice = specforge_ops::export::AGENT_FORMAT)]
    format: Format,
    /// Include coverage metadata in the response
    include_coverage: bool,
}

/// `specforge.query`: the query read view (`specforge_ops::query`), the
/// document `specforge query` prints for the same arguments; an unknown
/// kind of the filter rides in `_meta.diagnostics` (I020).
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let request = QueryRequest {
        entity_id: &args.entity_id,
        depth: Some(args.depth),
        kinds: args.kinds.iter().map(String::as_str).collect(),
        format: Some(args.format),
        include_coverage: args.include_coverage,
    };
    match query(&call.view(), &request) {
        Ok(outcome) => {
            let document: Value =
                serde_json::from_str(&outcome.document).expect("an export is JSON");
            ToolOutcome::ok(document).with_diagnostics(outcome.notices)
        }
        Err(error) => McpError::from(error).into(),
    }
}
