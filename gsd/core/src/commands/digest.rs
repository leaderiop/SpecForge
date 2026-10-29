//! `record_research`, `record_spec` and `record_validation` (D-r41-16, ADR 0299): each copies the computed `digest` of
//! its phase's context into its own `context_digest`, the value `gsd/W406` compares. The digest is the compute pass's:
//! the graph a command receives carries it, so no command hashes anything itself.

use serde_json::json;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run};
use crate::digest::scope_of;

/// The first context of the scope of `artifact` (by id, as the drift pass picks it) and its computed digest.
fn context_digest<'a>(
    graph: &'a CommandGraph,
    artifact: &GraphNode,
) -> Result<(&'a str, &'a str), String> {
    let scope = scope_of(&artifact.id);
    if scope.is_empty() {
        return Err(format!(
            "a project {} has no context",
            artifact.kind.replace('_', " ")
        ));
    }
    let context = graph
        .nodes_of_kind("context")
        .filter(|c| scope_of(&c.id) == scope)
        .min_by(|a, b| a.id.cmp(&b.id))
        .ok_or("the phase has no context; run /gsd-discuss-phase first")?;
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
        cmd.description("Record the digest of the phase's context in a research (context_digest), so a later change of the context is gsd/W406")
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
}
