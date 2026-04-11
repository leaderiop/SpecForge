use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required argument: entity_id"),
    };

    if state.graph.node(entity_id).is_none() {
        return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Entity not found: {}", entity_id));
    }

    // Trace the entity
    let chain = specforge_emitter::trace(&state.graph, entity_id);
    let affected: Vec<String> = match &chain {
        Ok(c) => {
            let mut ids: Vec<String> = c.upstream.iter().map(|l| l.entity_id.clone())
                .chain(c.downstream.iter().map(|l| l.entity_id.clone()))
                .chain(std::iter::once(entity_id.to_string()))
                .collect();
            ids.sort();
            ids.dedup();
            ids
        }
        Err(_) => vec![entity_id.to_string()],
    };

    // Find unverified entities in the trace
    let unverified: Vec<String> = affected.iter()
        .filter(|eid| {
            state.graph.node(eid)
                .map(|n| !matches!(n.fields.get("verify"), Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()))
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    let gaps = specforge_emitter::detect_trace_gaps(&state.graph);

    let result = serde_json::json!({
        "coverage_gaps": gaps,
        "unverified_entities": unverified,
        "affected_entities": affected
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": result.to_string()
            }
        }]
    }))
}
