use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit, generate_schema};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

/// `specforge://brief` — minimal graph (id, kind, title, edges). Query
/// parameters (C9-06): `root=<entity_id>` scopes to a subgraph, `depth=<n>`
/// bounds the traversal, `kinds=a,b` filters node kinds, `max_tokens=<n>`
/// budgets the payload. Scoped exports reference the published schema
/// (`schema_ref`) instead of embedding it (C6-07).
pub fn read(state: &McpState, uri: &str, id: Option<Value>) -> JsonRpcResponse {
    let (base, query) = crate::resources::split_query(uri);
    let parsed = crate::resources::parse_query(query);

    let schema = generate_schema(
        &state.kind_registry,
        &state.edge_registry,
        &state.field_registry,
        &state.extension_info,
    );

    let json_str = emit(
        &state.graph,
        &EmitOptions {
            format: EmitFormat::Brief,
            scope: parsed.root,
            depth: parsed.depth,
            kind_filter: parsed.kinds,
            token_budget: parsed.max_tokens,
            schema: Some(&schema),
            ..EmitOptions::default()
        },
    );

    match json_str {
        Ok(payload) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "contents": [{
                    "uri": base,
                    "mimeType": "application/json",
                    "text": payload
                }]
            }),
        ),
        Err(err) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, err.to_string()),
    }
}
