use crate::target::Call;
use crate::tool::{ErrorCode, McpError, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    entity_id: String,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let state = &*call.state;
    let entity_id = args.entity_id.as_str();

    let node = match state.graph().node(entity_id) {
        Some(n) => n,
        None => {
            return McpError::new(
                ErrorCode::EntityNotFound,
                format!("Entity not found: {entity_id}"),
            )
            .with_entity(entity_id)
            .into();
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
