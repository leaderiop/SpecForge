use serde::Deserialize;
use serde_json::Value;

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    entity_id: Option<String>,
    #[serde(default)]
    plan: Option<Value>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    if let Some(plan) = &args.plan {
        return plan_gaps(state, plan);
    }
    let entity_id = match args.entity_id.as_deref() {
        Some(e) => e,
        None => {
            return ToolOutcome::invalid_input(
                "entity_id",
                "Missing required parameter: entity_id or plan",
            );
        }
    };

    // The same expectations `specforge trace` uses, so both flag the same
    // missing links.
    let expectations = specforge_ops::trace::TraceExpectations::from_registries(
        &state.registries().fields,
        &state.registries().kinds,
    );
    match specforge_ops::trace::trace_with_expectations(state.graph(), entity_id, &expectations) {
        Ok(chain) => {
            let mut trace_val: serde_json::Value =
                match specforge_ops::trace::serialize_trace(&chain) {
                    Ok(json) => serde_json::from_str(&json).unwrap_or(serde_json::Value::Null),
                    Err(e) => return super::emitter_error(e, entity_id),
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

            ToolOutcome::ok(trace_val)
        }
        Err(err) => super::emitter_error(err, entity_id),
    }
}

/// Gap analysis of an agent plan against the graph, as an
/// `McpTracePlanResult`.
fn plan_gaps(state: &McpState, plan: &Value) -> ToolOutcome {
    let analysis = match analyze_plan(state, plan) {
        Ok(analysis) => analysis,
        Err(message) => return ToolOutcome::invalid_input("plan", message),
    };
    let body = serde_json::json!({
        "affected_entities": analysis.entries,
        "gaps": analysis.gaps,
    });
    ToolOutcome::ok(body)
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

    let testable: Vec<&str> =
        specforge_project::coverage::testable_kinds(&state.registries().kinds)
            .into_iter()
            .collect();
    let result = specforge_ops::plan::validate_plan(state.graph(), plan, &testable);
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
