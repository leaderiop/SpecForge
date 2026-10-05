use serde_json::Value;

use specforge_ops::plan::PlanError;
use specforge_ops::trace::Target;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::target::Call;

pub fn get(call: &Call<'_>, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let state: &McpState = call.state;
    let view = call.view();
    // A plan's entries, or the one entity, seed the trace.
    let (seeds, coverage_gaps, subject) = if let Some(plan) = args.get("plan") {
        match crate::tools::trace::analyze_plan(&view, plan) {
            Ok(analysis) => (
                analysis.entries,
                Value::from(analysis.gaps),
                "the plan".to_string(),
            ),
            Err(PlanError::NotAPlan(message)) => {
                return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, message);
            }
            // A prompt has no isError result: an unusable report is a
            // JSON-RPC error carrying the McpError the tools return.
            Err(PlanError::Report(e)) => {
                return JsonRpcResponse::error_with_data(
                    id,
                    error_codes::INTERNAL_ERROR,
                    e.to_string(),
                    crate::tools::coverage::report_mcp_error(&e, "specforge://prompts/trace")
                        .to_json(),
                );
            }
        }
    } else {
        let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => {
                return JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    "Missing required argument: plan or entity_id",
                );
            }
        };
        if state.graph().node(entity_id).is_none() {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Entity not found: {}", entity_id),
            );
        }
        let gaps = serde_json::to_value(specforge_ops::trace::detect_trace_gaps(state.graph()))
            .unwrap_or_default();
        (
            vec![entity_id.to_string()],
            gaps,
            format!("entity '{entity_id}'"),
        )
    };

    // Everything the seeds' trace chains reach, the seeds included.
    let mut affected: Vec<String> = seeds.clone();
    for seed in &seeds {
        if let Ok(outcome) = specforge_ops::trace::trace(&view, Target::Entity(seed)) {
            affected.extend(outcome.reached().into_iter().map(str::to_string));
        }
    }
    affected.sort();
    affected.dedup();

    // Find unverified entities in the trace
    let unverified: Vec<String> = affected
        .iter()
        .filter(|eid| {
            state
                .graph()
                .node(eid)
                .map(|n| specforge_graph::obligations(n).is_empty())
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    let result = serde_json::json!({
        "coverage_gaps": coverage_gaps,
        "unverified_entities": unverified,
        "affected_entities": affected
    });

    let instruction = format!(
        "Trace the dependency chain for {}. \
         The data below shows all affected entities, unverified entities in the chain, \
         and coverage gaps. Prioritize addressing unverified entities that are on critical paths.",
        subject
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
