use serde_json::Value;
use specforge_emitter::{
    EmitFormat, EmitOptions, emit, filter_graph_within_budget, generate_schema,
};

use crate::resources::{ReadOutcome, ResourceText, invalid_params};
use crate::state::McpState;

/// `specforge://graph` — full corpus, or scoped via query parameters
/// (C9-06): `?root=<entity_id>` scopes to a subgraph, `depth=<n>` bounds the
/// traversal, `kinds=a,b` filters node kinds, `max_tokens=<n>` budgets the
/// payload. Scoped exports reference the published schema (`schema_ref`)
/// instead of embedding it (C6-07).
pub fn read(state: &McpState, uri: &str) -> ReadOutcome {
    let (base, query) = crate::resources::split_query(uri);
    let parsed = crate::resources::parse_query(query);

    let schema = generate_schema(
        &state.kind_registry,
        &state.edge_registry,
        &state.field_registry,
        &state.extension_info,
    );

    let json_str = match parsed.max_tokens {
        // The schema-attached JSON path cannot budget internally (C1-10
        // budgets only the schemaless V1 path), so trim to the budget first,
        // then emit the trimmed graph with the schema attached.
        Some(budget) => {
            let trimmed = filter_graph_within_budget(&state.graph, budget, |g| {
                emit(
                    g,
                    &EmitOptions {
                        format: EmitFormat::Json,
                        scope: parsed.root,
                        depth: parsed.depth,
                        kind_filter: parsed.kinds.clone(),
                        ..EmitOptions::default()
                    },
                )
            });
            match trimmed {
                Ok(graph) => emit(
                    &graph,
                    &EmitOptions {
                        format: EmitFormat::Json,
                        scope: parsed.root,
                        depth: parsed.depth,
                        kind_filter: parsed.kinds,
                        schema: Some(&schema),
                        ..EmitOptions::default()
                    },
                ),
                Err(err) => Err(err),
            }
        }
        None => emit(
            &state.graph,
            &EmitOptions {
                format: EmitFormat::Json,
                scope: parsed.root,
                depth: parsed.depth,
                kind_filter: parsed.kinds,
                schema: Some(&schema),
                ..EmitOptions::default()
            },
        ),
    };

    match json_str {
        Ok(payload) => {
            let contents: Value =
                serde_json::from_str(&payload).expect("graph emit always produces JSON");
            Ok(ResourceText::json(base, contents.to_string()))
        }
        Err(err) => Err(invalid_params(err.to_string())),
    }
}
