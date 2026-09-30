use serde_json::Value;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, args: Value) -> ToolOutcome {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return ToolOutcome::invalid_params("Missing required parameter: entity_id");
        }
    };

    let node = match state.graph.node(entity_id) {
        Some(n) => n,
        None => {
            return ToolOutcome::failed(format!("Entity not found: {}", entity_id));
        }
    };

    let result = serde_json::json!({
        "entity_id": node.id.raw,
        "file_path": node.source_span.file,
        "line": node.source_span.start_line,
        "column": node.source_span.start_col
    });

    ToolOutcome::ok(result)
}
