use serde::Deserialize;
use serde_json::{Value, json};
use specforge_ops::plan::PlanError;
use specforge_ops::trace::{Gap, Target};
use specforge_ops::view::ProjectView;

use crate::args::lenient;
use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    entity_id: Option<String>,
    #[serde(default)]
    plan: Option<Value>,
}

/// `specforge.trace`: the trace operation over the served project. An
/// entity's result is the document `specforge trace <entity> --format
/// json` writes; a plan's is an `McpTracePlanResult`.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let view = call.view();
    if let Some(plan) = &args.plan {
        return plan_gaps(&view, plan);
    }
    let Some(entity_id) = args.entity_id.as_deref() else {
        return ToolOutcome::invalid_input(
            "entity_id",
            "Missing required parameter: entity_id or plan",
        );
    };
    match specforge_ops::trace::trace(&view, Target::Entity(entity_id)) {
        Ok(outcome) => match serde_json::to_value(&outcome) {
            Ok(document) => ToolOutcome::ok(document),
            Err(e) => ToolOutcome::error(
                crate::tool::ErrorCode::InternalError,
                format!("trace serialization failed: {e}"),
            ),
        },
        Err(error) => crate::operations::op_error(error.into())
            .with_entity(entity_id)
            .into(),
    }
}

/// Gap analysis of an agent plan against the graph, as an
/// `McpTracePlanResult`.
fn plan_gaps(view: &ProjectView, plan: &Value) -> ToolOutcome {
    match analyze_plan(view, plan) {
        Ok(analysis) => ToolOutcome::ok(json!({
            "affected_entities": analysis.entries,
            "gaps": analysis.gaps,
        })),
        Err(PlanError::NotAPlan(why)) => ToolOutcome::invalid_input("plan", why),
        Err(PlanError::Report(error)) => super::coverage::report_error_result(&error),
    }
}

/// A gap as MCP spells it (`McpTraceGap`).
pub(crate) fn gap_json(gap: &Gap) -> Value {
    json!({
        "source_entity": gap.source(),
        "target_entity": gap.target(),
        "missing_link_type": gap.kind(),
        "gap_context": gap.context(),
    })
}

/// An agent plan checked against the graph (`specforge_ops::plan::check`).
pub(crate) struct PlanAnalysis {
    /// Plan entries that name an entity in the graph, in plan order.
    pub entries: Vec<String>,
    /// `McpTraceGap`s: unresolved entries, missing entries, bad ordering.
    pub gaps: Vec<Value>,
}

/// Check `plan` — an `AgentPlan` object, or JSON text of one — against the
/// view's graph.
pub(crate) fn analyze_plan(view: &ProjectView, plan: &Value) -> Result<PlanAnalysis, PlanError> {
    let outcome = specforge_ops::plan::check(view, plan)?;
    Ok(PlanAnalysis {
        entries: outcome.entries,
        gaps: outcome
            .gaps
            .into_iter()
            .map(|gap| gap_json(&Gap::Plan(gap)))
            .collect(),
    })
}
