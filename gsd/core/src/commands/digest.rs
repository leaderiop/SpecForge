//! `record_research`, `record_spec` and `record_validation` (D-r41-16, ADR 0299): each copies the computed `digest` of
//! its phase's (a quick research: its quick plan's) context into its own `context_digest`, the value `gsd/W406` compares. The digest is the compute pass's:
//! the graph a command receives carries it, so no command hashes anything itself.
//!
//! Also ports gsd-core bin/lib modules: context-composer.cjs, init.cjs.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run};
use crate::digest::scope_of;

/// The quick plan `id` names in `plan`, through its resolved edge.
fn quick_plan<'a>(graph: &'a CommandGraph, id: &str) -> Option<&'a str> {
    graph
        .edges_from(id)
        .into_iter()
        .find(|e| e.label == "plan")
        .map(|e| e.target.as_str())
}

/// The first context of the owner of `artifact` (its quick plan, else its phase scope; by id, as the drift pass picks
/// it) and its computed digest.
fn context_digest<'a>(
    graph: &'a CommandGraph,
    artifact: &GraphNode,
) -> Result<(&'a str, &'a str), String> {
    let quick = quick_plan(graph, &artifact.id);
    let scope = scope_of(&artifact.id);
    if quick.is_none() && scope.is_empty() {
        return Err(format!(
            "a project {} has no context",
            artifact.kind.replace('_', " ")
        ));
    }
    let context = graph
        .nodes_of_kind("context")
        .filter(|c| match quick {
            Some(plan) => quick_plan(graph, &c.id) == Some(plan),
            None => scope_of(&c.id) == scope,
        })
        .min_by(|a, b| a.id.cmp(&b.id))
        .ok_or(if quick.is_some() {
            "the quick plan has no context; run /gsd-quick --discuss first"
        } else {
            "the phase has no context; run /gsd-discuss-phase first"
        })?;
    let digest = context.text("digest").ok_or_else(|| {
        format!(
            "the graph carries no computed digest for {}; run `specforge check`",
            context.id
        )
    })?;
    Ok((&context.id, digest))
}

fn record(call: &CommandCall<'_>, kind: &str) -> Result<CommandOutput, String> {
    let artifact = node(call, kind, required(call, "entity")?)?;
    let (context, digest) = context_digest(call.graph(), artifact)?;
    let mut plan = call.plan();
    plan.set(&artifact.id, "context_digest", json!(digest));
    Ok(answer(
        call,
        json!({ "wrote": [artifact.id], "context": context, "context_digest": digest }),
        format!("{} recorded against {context}: {digest}\n", artifact.id),
        Some(&plan),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("record_research", |cmd| {
        cmd.description("Record the digest of the phase's or quick plan's context in a research (context_digest), so a later change of the context is gsd/W406")
            .writes()
            .arg("entity", |a| {
                a.required().description("the research (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, |c| record(c, "research")));
    });
    c.command("record_spec", |cmd| {
        cmd.description("Record the digest of the phase's context in a phase spec (context_digest), so a later change of the context is gsd/W406")
            .writes()
            .arg("entity", |a| {
                a.required().description("the phase spec (its path, or a short form that is not ambiguous)");
            })
            .handler(|call| run(call, |c| record(c, "phase_spec")));
    });
    c.command("record_validation", |cmd| {
        cmd.description("Record the digest of the phase's context in a validation (context_digest), so a later change of the context is gsd/W406")
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

    fn node(id: &str, kind: &str, digest: Option<&str>) -> GraphNode {
        let mut node = GraphNode {
            id: id.to_string(),
            kind: kind.to_string(),
            ..Default::default()
        };
        if let Some(digest) = digest {
            node.fields.insert("digest".into(), json!(digest));
        }
        node
    }

    #[test]
    fn an_uncomputed_digest_is_refused() {
        let research = node("v1.0/02/02-RESEARCH", "research", None);
        let graph = CommandGraph::new(
            vec![
                node("v1.0/02/02-CONTEXT", "context", None),
                research.clone(),
            ],
            Vec::new(),
        );
        let why = context_digest(&graph, &research).unwrap_err();
        assert!(
            why.contains("no computed digest for v1.0/02/02-CONTEXT"),
            "{why}"
        );
        assert!(why.contains("specforge check"), "{why}");
    }

    #[test]
    fn the_digest_is_the_contexts_in_the_same_phase() {
        let research = node("v1.0/02/02-RESEARCH", "research", None);
        let graph = CommandGraph::new(
            vec![
                node("v1.0/01/01-CONTEXT", "context", Some("fnv1a64:1")),
                node("v1.0/02/02-CONTEXT", "context", Some("fnv1a64:2")),
                research.clone(),
            ],
            Vec::new(),
        );
        assert_eq!(
            context_digest(&graph, &research),
            Ok(("v1.0/02/02-CONTEXT", "fnv1a64:2"))
        );
    }

    #[test]
    fn a_quick_research_records_its_own_quick_plans_context() {
        let edge = |source: &str, plan: &str| GraphEdge {
            source: source.into(),
            target: plan.into(),
            label: "plan".into(),
        };
        let research = node("251101-abc-RESEARCH", "research", None);
        let graph = CommandGraph::new(
            vec![
                node("251101-aaa-CONTEXT", "context", Some("fnv1a64:1")),
                node("251101-abc-CONTEXT", "context", Some("fnv1a64:2")),
                research.clone(),
            ],
            vec![
                edge("251101-aaa-CONTEXT", "251101-aaa"),
                edge("251101-abc-CONTEXT", "251101-abc"),
                edge("251101-abc-RESEARCH", "251101-abc"),
            ],
        );
        assert_eq!(
            context_digest(&graph, &research),
            Ok(("251101-abc-CONTEXT", "fnv1a64:2"))
        );
        let lone = CommandGraph::new(
            vec![research.clone()],
            vec![edge("251101-abc-RESEARCH", "251101-abc")],
        );
        let why = context_digest(&lone, &research).unwrap_err();
        assert!(why.contains("/gsd-quick --discuss"), "{why}");
    }
}
