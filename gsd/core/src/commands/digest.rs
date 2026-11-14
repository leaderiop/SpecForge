//! `record_research`, `record_spec` and `record_validation` (ADR 0356): each asks the host to record, in the entity's
//! `context_pack`, the pack of `gsd/phase-context` (a quick plan's research: `gsd/quick-context`) it was written
//! against. The host computes the pack and its digest, so no command hashes anything, and reports the record
//! `gsd/W406` when that pack moves.
//!
//! Also ports gsd-core bin/lib modules: context-composer.cjs, init.cjs.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run};

/// Whether `id` names an entity through its edge `label`.
fn names(graph: &CommandGraph, id: &str, label: &str) -> bool {
    graph.edges_from(id).into_iter().any(|e| e.label == label)
}

/// The route whose pack `artifact` records: a quick plan's research reads its quick plan's context, any other its
/// phase's; the project's research has none.
fn route(graph: &CommandGraph, artifact: &GraphNode) -> Result<&'static str, String> {
    if names(graph, &artifact.id, "plan") {
        Ok("gsd/quick-context")
    } else if names(graph, &artifact.id, "phase") {
        Ok("gsd/phase-context")
    } else {
        Err(format!(
            "a project {} has no context",
            artifact.kind.replace('_', " ")
        ))
    }
}

fn record(call: &CommandCall<'_>, kind: &str) -> Result<CommandOutput, String> {
    let artifact = node(call, kind, required(call, "entity")?)?;
    let route = route(call.graph(), artifact)?;
    let mut plan = call.plan();
    plan.record_context(&artifact.id, route, None);
    Ok(answer(
        call,
        json!({ "recorded": artifact.id, "route": route }),
        format!("{} records the pack of {route}\n", artifact.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("record_research", |cmd| {
        cmd.description("Record the pack of gsd/phase-context (gsd/quick-context for a quick plan's research) on a research, so a later change of its context is gsd/W406")
            .writes()
            .arg("entity", |a| {
                a.required().description("the research (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, |c| record(c, "research")));
    });
    c.command("record_spec", |cmd| {
        cmd.description("Record the pack of gsd/phase-context on a phase spec, so a later change of its context is gsd/W406")
            .writes()
            .arg("entity", |a| {
                a.required().description("the phase spec (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, |c| record(c, "phase_spec")));
    });
    c.command("record_validation", |cmd| {
        cmd.description("Record the pack of gsd/phase-context on a validation, so a later change of its context is gsd/W406")
            .writes()
            .arg("entity", |a| {
                a.required().description("the validation (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, |c| record(c, "validation")));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, kind: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            ..Default::default()
        }
    }

    fn edge(source: &str, target: &str, label: &str) -> GraphEdge {
        GraphEdge {
            source: source.into(),
            target: target.into(),
            label: label.into(),
        }
    }

    #[test]
    fn a_phase_research_records_the_phase_context_and_a_quick_one_the_quick_context() {
        let phase = node("v1.0/02/02-RESEARCH", "research");
        let quick = node("251101-abc-RESEARCH", "research");
        let graph = CommandGraph::new(
            vec![phase.clone(), quick.clone()],
            vec![
                edge(&phase.id, "v1.0/02", "phase"),
                edge(&quick.id, "251101-abc", "plan"),
            ],
        );
        assert_eq!(route(&graph, &phase), Ok("gsd/phase-context"));
        assert_eq!(route(&graph, &quick), Ok("gsd/quick-context"));
    }

    #[test]
    fn a_project_research_has_no_context() {
        let research = node("RESEARCH", "research");
        let graph = CommandGraph::new(vec![research.clone()], Vec::new());
        assert_eq!(
            route(&graph, &research),
            Err("a project research has no context".to_string())
        );
    }
}
