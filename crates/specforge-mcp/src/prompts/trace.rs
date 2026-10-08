//! `specforge://prompts/trace`: the dependency chain of a plan or of one
//! entity, and what in it is unverified.

use std::collections::BTreeSet;

use serde_json::json;
use specforge_ops::plan::PlanError;
use specforge_ops::trace::{Target, trace};

use crate::args::{AgentPlan, Arguments};
use crate::prompt::{PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError};
use crate::tools::trace::{analyze_plan, gap_json};

/// `specforge://prompts/trace`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// AgentPlan JSON ({"entries": [{"entity_id", "action"}]}) to check against the graph
    plan: Option<AgentPlan>,
    /// Entity ID to trace when no plan is given
    entity_id: Option<String>,
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let view = call.view();
    // A plan's entries, or the one entity, seed the trace; the gaps are the
    // plan's, or the entity's chain's missing links.
    let (seeds, coverage_gaps, subject) = match (&args.plan, args.entity_id.as_deref()) {
        (Some(plan), _) => {
            let analysis = analyze_plan(&view, &plan.0).map_err(|error| match error {
                PlanError::NotAPlan(why) => {
                    McpError::new(ErrorCode::InvalidInput, why).with_argument("plan")
                }
                PlanError::Report(e) => McpError::from(e),
            })?;
            (analysis.entries, analysis.gaps, "the plan".to_string())
        }
        (None, Some(entity_id)) => {
            // As the trace tool traces it: its chain and missing links.
            let outcome = trace(&view, Target::Entity(entity_id))
                .map_err(|error| McpError::from(specforge_ops::OpError::from(error)))?;
            let gaps = outcome.gaps().iter().map(gap_json).collect();
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
    let mut affected: BTreeSet<String> = seeds.iter().cloned().collect();
    for seed in &seeds {
        if let Ok(outcome) = trace(&view, Target::Entity(seed)) {
            affected.extend(outcome.reached().into_iter().map(str::to_string));
        }
    }

    // The affected entities that count toward coverage and are not proven
    // (the coverage view's one definition of unverified).
    let coverage = view.coverage().map_err(McpError::from)?;
    let unverified: Vec<&String> = affected
        .iter()
        .filter(|id| coverage.is_unverified(id))
        .collect();

    let payload = json!({
        "coverage_gaps": coverage_gaps,
        "unverified_entities": unverified,
        "affected_entities": affected,
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
