use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    if let Some(plan) = args.get("plan") {
        return plan_gaps(state, plan, id);
    }
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: entity_id or plan",
            );
        }
    };

    match specforge_emitter::trace(&state.graph, entity_id) {
        Ok(chain) => {
            let mut trace_val: serde_json::Value = match specforge_emitter::serialize_trace(&chain)
            {
                Ok(json) => serde_json::from_str(&json).unwrap_or(serde_json::Value::Null),
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        crate::protocol::error_codes::INTERNAL_ERROR,
                        e.to_string(),
                    );
                }
            };

            // Add gaps detection
            let mut gaps = Vec::new();
            if chain.upstream.is_empty() {
                gaps.push("no upstream links");
            }
            if chain.downstream.is_empty() {
                gaps.push("no downstream links");
            }
            if let Some(obj) = trace_val.as_object_mut() {
                obj.insert("gaps".into(), serde_json::json!(gaps));
            }

            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": trace_val.to_string()
                    }]
                }),
            )
        }
        Err(err) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, err.to_string()),
    }
}

/// Gap analysis of an agent plan (`{"entries": [{"entity_id"}]}`) against
/// the graph, by `validate_plan`, as an `McpTracePlanResult`.
fn plan_gaps(state: &McpState, plan: &Value, id: Option<Value>) -> JsonRpcResponse {
    let testable: Vec<&str> = state
        .kind_registry
        .iter()
        .filter(|(_, entry)| entry.testable)
        .map(|(kind, _)| kind.as_str())
        .collect();
    let result = specforge_emitter::validate_plan(&state.graph, plan, &testable);
    let gaps: Vec<Value> = result
        .gaps
        .iter()
        .map(|gap| {
            serde_json::json!({
                "source_entity": gap.source,
                "target_entity": gap.target,
                "missing_link_type": gap.kind.as_str(),
                "gap_context": gap.context,
            })
        })
        .collect();
    let body = serde_json::json!({
        "affected_entities": result.validated_entries,
        "gaps": gaps,
    });
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": body.to_string()
            }]
        }),
    )
}
