//! `specforge://prompts/context`: what implementing one entity needs.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::prompt::{PromptArgs, PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::{ErrorCode, McpError, entity_not_found};

#[derive(Debug, Deserialize)]
pub struct Args {
    entity_id: String,
    #[serde(default, deserialize_with = "crate::args::id_list")]
    structural_constraints: Vec<String>,
}

impl PromptArgs for Args {
    const DESCRIPTIONS: &'static [(&'static str, &'static str)] = &[
        ("entity_id", "Entity ID to get context for"),
        (
            "structural_constraints",
            "Entity IDs to include as context even when not connected (array or comma-separated)",
        ),
    ];
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let view = call.view();
    let graph = view.graph;
    let entity_id = args.entity_id.as_str();
    let node = graph
        .node(entity_id)
        .ok_or_else(|| entity_not_found(entity_id))?;

    // The statement the extension declares (headline and normative), e.g.
    // a behavior's `contract`; empty for a kind that declares none.
    let contract_text =
        specforge_emitter::context::headline_statement(node, &view.registries.fields)
            .unwrap_or_default();

    let upstream: Vec<String> = graph
        .edges_to(entity_id)
        .iter()
        .map(|e| e.source.to_string())
        .collect();

    let downstream: Vec<String> = graph
        .edges_from(entity_id)
        .iter()
        .map(|e| e.target.to_string())
        .collect();

    let verify_expectations: Vec<String> = specforge_graph::obligations(node)
        .iter()
        .map(|s| format!("{} {}", s.kind, s.description))
        .collect();

    // Structural constraints: entities the caller wants in the context even
    // when no edge connects them to this one.
    let mut constraint_entities: Vec<Value> = Vec::new();
    for constraint_id in &args.structural_constraints {
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
        "structural_constraints": args.structural_constraints,
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
