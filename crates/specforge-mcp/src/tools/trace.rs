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

    // The same expectations `specforge trace` uses, so both flag the same
    // missing links.
    let expectations = specforge_emitter::TraceExpectations::from_registries(
        &state.field_registry,
        &state.kind_registry,
    );
    match specforge_emitter::trace_with_expectations(&state.graph, entity_id, &expectations) {
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

/// Gap analysis of an agent plan against the graph, as an
/// `McpTracePlanResult`.
fn plan_gaps(state: &McpState, plan: &Value, id: Option<Value>) -> JsonRpcResponse {
    let analysis = match analyze_plan(state, plan) {
        Ok(analysis) => analysis,
        Err(message) => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, message),
    };
    let body = serde_json::json!({
        "affected_entities": analysis.entries,
        "gaps": analysis.gaps,
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

/// An agent plan checked against the graph by `validate_plan`.
pub(crate) struct PlanAnalysis {
    /// Plan entries that name an entity in the graph, in plan order.
    pub entries: Vec<String>,
    /// `McpTraceGap`s: unresolved entries, missing entries, bad ordering.
    pub gaps: Vec<Value>,
}

/// Check `plan` — an `AgentPlan` object, or JSON text of one — against the
/// graph. `Err` describes why it isn't a plan.
pub(crate) fn analyze_plan(state: &McpState, plan: &Value) -> Result<PlanAnalysis, String> {
    let parsed;
    let plan = match plan {
        Value::String(text) => {
            parsed = serde_json::from_str::<Value>(text)
                .map_err(|e| format!("plan is not valid JSON: {e}"))?;
            &parsed
        }
        other => other,
    };
    let Some(entries) = plan.get("entries").and_then(|e| e.as_array()) else {
        return Err("plan must be an AgentPlan object with an entries array".into());
    };
    for (i, entry) in entries.iter().enumerate() {
        if !entry.get("entity_id").is_some_and(|v| v.is_string()) {
            return Err(format!("plan.entries[{i}].entity_id must be a string"));
        }
    }

    let testable: Vec<&str> = state
        .kind_registry
        .iter()
        .filter(|(_, entry)| entry.testable)
        .map(|(kind, _)| kind.as_str())
        .collect();
    let result = specforge_emitter::validate_plan(&state.graph, plan, &testable);
    let gaps = result
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
    Ok(PlanAnalysis {
        entries: result.validated_entries,
        gaps,
    })
}
