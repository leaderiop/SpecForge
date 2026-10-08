use serde_json::{Value, json};
use specforge_ops::plan::PlanError;
use specforge_ops::trace::{Gap, Target};
use specforge_ops::view::ProjectView;

use crate::args::{AgentPlan, Arguments};
use crate::tool::ToolOutcome;

/// `specforge.trace`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to trace
    entity_id: Option<String>,
    /// An agent plan, {"entries": [{"entity_id": ...}]}, to check for gaps against the graph instead of tracing one entity
    plan: Option<AgentPlan>,
}

/// `specforge.trace`: the trace operation over the served project. An
/// entity's result is the document `specforge trace <entity> --format
/// json` writes; a plan's is an `McpTracePlanResult`.
pub fn call(view: ProjectView<'_>, args: Args) -> ToolOutcome {
    if let Some(plan) = &args.plan {
        return plan_gaps(&view, &plan.0);
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
        Err(error) => crate::tool::McpError::from(specforge_ops::OpError::from(error))
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
        Err(PlanError::Report(error)) => crate::tool::McpError::from(error).into(),
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
