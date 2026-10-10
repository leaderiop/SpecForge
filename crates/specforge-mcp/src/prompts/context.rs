//! `specforge://prompts/context`: what implementing one entity needs.

use serde_json::{Value, json};

use crate::args::{Arguments, EntityIds};
use crate::prompt::{PromptOutcome, Rendered};
use crate::tool::{ErrorCode, McpError};
use specforge_ops::view::ProjectView;

/// `specforge://prompts/context`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to get context for
    entity_id: String,
    /// Entity IDs to include as context even when not connected (array or comma-separated)
    structural_constraints: EntityIds,
}

pub fn render(view: ProjectView<'_>, args: Args) -> PromptOutcome {
    let graph = view.graph();
    let entity_id = args.entity_id.as_str();
    // The inspect read view, without the reported diagnostics (the prompt
    // shows none) and whatever its coverage (a report that cannot be read
    // does not fail the prompt).
    let facts =
        specforge_ops::inspect::inspect(&view.reporting(&[]), entity_id).map_err(McpError::from)?;
    let node = facts.node;

    // The statement the extension declares (headline and normative), e.g.
    // a behavior's `contract`; empty for a kind that declares none.
    let contract_text = facts.headline.unwrap_or_default();
    let upstream: Vec<&str> = facts
        .references
        .incoming
        .iter()
        .map(|r| r.peer.as_str())
        .collect();
    let downstream: Vec<&str> = facts
        .references
        .outgoing
        .iter()
        .map(|r| r.peer.as_str())
        .collect();
    let verify_expectations: Vec<String> = facts
        .obligations
        .iter()
        .map(specforge_ops::inspect::obligation_text)
        .collect();

    // Structural constraints: entities the caller wants in the context even
    // when no edge connects them to this one.
    let mut constraint_entities: Vec<Value> = Vec::new();
    for constraint_id in &args.structural_constraints.0 {
        let Some(constraint) = graph.node(constraint_id) else {
            return Err(Box::new(
                McpError::new(
                    ErrorCode::EntityNotFound,
                    format!("Structural constraint entity not found: {constraint_id}"),
                )
                .with_entity(constraint_id.as_str())
                .with_argument("structural_constraints"),
            ));
        };
        constraint_entities.push(json!({
            "entity_id": constraint.id.raw,
            "kind": constraint.kind.raw,
            "title": constraint.title,
            "fields": specforge_emitter::field_map_to_json(&constraint.fields),
        }));
    }

    let payload = json!({
        "structural_constraints": args.structural_constraints.0,
        "structural_constraint_entities": constraint_entities,
        "entity_id": entity_id,
        "kind": node.kind.raw,
        "contract_text": contract_text,
        // Every field, whatever the kind names its text: an invariant's
        // `guarantee`, a decision's `rationale`.
        "fields": specforge_emitter::field_map_to_json(&node.fields),
        "upstream_entities": upstream,
        "downstream_entities": downstream,
        "verify_expectations": verify_expectations
    });

    let instruction = format!(
        "You are implementing the entity '{}' (kind: {}). \
         Use the structured context below to guide your implementation. \
         Respect the contract (or the guarantee, rationale or other text in its fields), satisfy verify expectations, and consider upstream/downstream dependencies.",
        entity_id, node.kind.raw
    );

    Ok(Rendered {
        instruction,
        payload,
    })
}
