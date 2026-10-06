use specforge_emitter::{EmitFormat, EmitOptions, EmitterError, emit};

use crate::resources::{ReadOutcome, ResourceText, entity_not_found, invalid_params};
use crate::state::McpState;

/// `specforge://context` and `specforge://context/{entity_id}` —
/// context-optimized graph (contract, status, verify). Query parameters
/// (C9-06): `root=<entity_id>` (or the path entity id) scopes to a subgraph,
/// `depth=<n>` bounds the traversal, `kinds=a,b` filters node kinds,
/// `max_tokens=<n>` budgets the payload. Scoped exports reference the
/// published schema (`schema_ref`) instead of embedding it (C6-07).
pub fn read(state: &McpState, uri: &str) -> ReadOutcome {
    let (base, query) = crate::resources::split_query(uri);
    let mut parsed = crate::resources::parse_query(query);
    if parsed.root.is_none() {
        parsed.root = base
            .strip_prefix("specforge://context/")
            .filter(|root| !root.is_empty());
    }

    let json_str = emit(
        state.graph(),
        &EmitOptions {
            format: EmitFormat::Context,
            scope: parsed.root,
            depth: parsed.depth,
            kind_filter: parsed.kinds,
            token_budget: parsed.max_tokens,
            // The entities only: the schema (specforge://schema) is most of
            // the bytes and an agent reading the context needs the graph.
            schema: None,
            field_registry: Some(&state.registries().fields),
            ..EmitOptions::default()
        },
    );

    match json_str {
        Ok(payload) => Ok(ResourceText::json(payload)),
        Err(EmitterError::EntityNotFound(message)) => {
            Err(entity_not_found(message, parsed.root.unwrap_or_default()))
        }
        Err(err) => Err(invalid_params(err.to_string())),
    }
}
