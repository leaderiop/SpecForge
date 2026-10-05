//! `specforge://prompts/trace`: the dependency chain of a plan or of one
//! entity, and what in it is unverified.

use serde::Deserialize;
use serde_json::{Value, json};

use specforge_ops::plan::PlanError;
use specforge_ops::trace::Target;

use crate::prompt::{PromptArgs, PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError, entity_not_found};

#[derive(Debug, Deserialize)]
pub struct Args {
    /// An `AgentPlan` object, or JSON text of one.
    #[serde(default)]
    plan: Option<Value>,
    #[serde(default)]
    entity_id: Option<String>,
}

impl PromptArgs for Args {
    const DESCRIPTIONS: &'static [(&'static str, &'static str)] = &[
        (
            "plan",
            "AgentPlan JSON ({\"entries\": [{\"entity_id\", \"action\"}]}) to check against the graph",
        ),
        ("entity_id", "Entity ID to trace when no plan is given"),
    ];
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let view = call.view();
    let graph = view.graph;
    // A plan's entries, or the one entity, seed the trace.
    let (seeds, coverage_gaps, subject) = match (&args.plan, args.entity_id.as_deref()) {
        (Some(plan), _) => {
            let analysis =
                crate::tools::trace::analyze_plan(&view, plan).map_err(|error| match error {
                    PlanError::NotAPlan(why) => {
                        McpError::new(ErrorCode::InvalidInput, why).with_argument("plan")
                    }
                    PlanError::Report(e) => crate::tools::coverage::report_mcp_error(&e),
                })?;
            (
                analysis.entries,
                Value::from(analysis.gaps),
                "the plan".to_string(),
            )
        }
        (None, Some(entity_id)) => {
            if graph.node(entity_id).is_none() {
                return Err(entity_not_found(entity_id).into());
            }
            let gaps = serde_json::to_value(specforge_ops::trace::detect_trace_gaps(graph))
                .unwrap_or_default();
            (
                vec![entity_id.to_string()],
                gaps,
                format!("entity '{entity_id}'"),
            )
        }
        (None, None) => {
            // As the trace tool refuses it.
            return Err(Box::new(
                McpError::new(
                    ErrorCode::InvalidInput,
                    "Missing required parameter: entity_id or plan",
                )
                .with_argument("entity_id"),
            ));
        }
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
            graph
                .node(eid)
                .map(|n| specforge_graph::obligations(n).is_empty())
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    let payload = json!({
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

    Ok(Rendered {
        instruction,
        payload,
    })
}
