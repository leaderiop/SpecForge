use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required argument: entity_id",
            );
        }
    };

    let node = match state.graph().node(entity_id) {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Entity not found: {}", entity_id),
            );
        }
    };

    // The statement the extension declares (headline and normative), e.g.
    // a behavior's `contract`; empty for a kind that declares none.
    let contract_text =
        specforge_emitter::context::headline_statement(node, &state.registries().fields)
            .unwrap_or_default();

    let upstream: Vec<String> = state
        .graph()
        .edges_to(entity_id)
        .iter()
        .map(|e| e.source.to_string())
        .collect();

    let downstream: Vec<String> = state
        .graph()
        .edges_from(entity_id)
        .iter()
        .map(|e| e.target.to_string())
        .collect();

    let verify_expectations: Vec<String> = specforge_graph::obligations(node)
        .iter()
        .map(|s| format!("{} {}", s.kind, s.description))
        .collect();

    // Structural constraints: entities the caller wants in the context even
    // when no edge connects them to this one.
    let mut constraint_ids: Vec<String> = Vec::new();
    let mut constraint_entities: Vec<Value> = Vec::new();
    // MCP prompt arguments are strings, so a comma-separated list works too.
    let requested: Vec<&str> = match args.get("structural_constraints") {
        Some(Value::Array(ids)) => ids.iter().filter_map(|v| v.as_str()).collect(),
        Some(Value::String(ids)) => ids
            .split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    for constraint_id in requested {
        let Some(constraint) = state.graph().node(constraint_id) else {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Structural constraint entity not found: {constraint_id}"),
            );
        };
        constraint_ids.push(constraint_id.to_string());
        constraint_entities.push(serde_json::json!({
            "entity_id": constraint.id.raw,
            "kind": constraint.kind.raw,
            "title": constraint.title,
            "fields": specforge_emitter::field_map_to_json(&constraint.fields),
        }));
    }

    let result = serde_json::json!({
        "structural_constraints": constraint_ids,
        "structural_constraint_entities": constraint_entities,
        "entity_id": entity_id,
        "kind": node.kind.raw,
        "contract_text": contract_text,
        // Every field, whatever the kind names its text: an invariant's
        // `guarantee`, a decision's `rationale`.
        "fields": specforge_emitter::field_map_to_json(&node.fields),
        "upstream_entities": upstream,
        "downstream_entities": downstream,
        "verify_expectations": verify_expectations
    });

    let instruction = format!(
        "You are implementing the entity '{}' (kind: {}). \
         Use the structured context below to guide your implementation. \
         Respect the contract (or the guarantee, rationale or other text in its fields), satisfy verify expectations, and consider upstream/downstream dependencies.",
        entity_id, node.kind.raw
    );

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": { "type": "text", "text": instruction }
                },
                {
                    "role": "assistant",
                    "content": { "type": "text", "text": result.to_string() }
                }
            ]
        }),
    )
}
