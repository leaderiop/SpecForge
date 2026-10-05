use serde_json::Value;

use crate::state::McpState;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let entity_id = args.entity_id.as_str();

    if state.graph().node(entity_id).is_none() {
        return McpError::new(
            ErrorCode::EntityNotFound,
            format!("Entity not found: {entity_id}"),
        )
        .with_entity(entity_id)
        .into();
    }

    let locations: Vec<Value> = state
        .graph()
        .edges_to(entity_id)
        .iter()
        .filter_map(|edge| {
            state.graph().node(edge.source.as_str()).map(|n| {
                serde_json::json!({
                    "referencing_entity_id": n.id.raw,
                    "source_span": {
                        "file": n.source_span.file,
                        "start_line": n.source_span.start_line,
                        "start_col": n.source_span.start_col,
                        "end_line": n.source_span.end_line,
                        "end_col": n.source_span.end_col,
                    }
                })
            })
        })
        .collect();

    let result = serde_json::json!({
        "entity_id": entity_id,
        "locations": locations
    });

    ToolOutcome::ok(result)
}
