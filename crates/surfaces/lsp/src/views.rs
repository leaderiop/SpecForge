//! The requests only SpecForge's editor client asks (ADR 0268): the session's graph for its tree and
//! its graph view, and the entity under its cursor. Both answer from the session the other requests
//! read, buffers included.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_ops::read::export::{self, Format, Schema};
use specforge_ops::read::query::{self, QueryRequest};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{Position, Url};

use crate::LspState;
use crate::answers::entity_under_cursor;
use crate::navigation::{Compiled, symbol_kind_from_entity};

/// The session's graph with each of its kinds' symbol kind (ADR 0268 D1).
pub const GRAPH: &str = "specforge/graph";
/// The entity the cursor names, as the graph writes it (ADR 0268 D1).
pub const ENTITY: &str = "specforge/entity";
/// Sent after each rebuild to a client that declared [`GRAPH_CHANGED_CAPABILITY`] (ADR 0268 D2).
pub const GRAPH_CHANGED: &str = "specforge/graphChanged";
/// The experimental client capability that asks for [`GRAPH_CHANGED`].
pub const GRAPH_CHANGED_CAPABILITY: &str = "specforgeGraphChanged";

fn failed(message: impl std::fmt::Display) -> Error {
    let mut error = Error::internal_error();
    error.message = message.to_string().into();
    error
}

/// `{ "graph": <Graph Protocol 1.0 document>, "kinds": { <kind>: <SymbolKind> }, "degrees": { <id>: <n> } }`:
/// the export `specforge export --format=graph --no-schema` writes, over the session (its open buffers
/// included, the last complete compile while one is out for an update), the symbol kind of every kind
/// its nodes hold ([`symbol_kind_from_entity`]), and each node's edges to and from *other* entities
/// (`Graph::degree(id).total()`: a self-edge counts for nothing, ADR 0174 D2). Nodes are the compile's:
/// a file typed since keeps its compiled lines.
pub fn graph(state: &LspState) -> Result<Value> {
    let exported = export::export(
        &state.view(),
        &export::Request {
            format: Some(Format::Graph),
            schema: Schema::Without,
            ..Default::default()
        },
    )
    .map_err(|refusal| failed(refusal.error.message))?;
    let graph: Value =
        serde_json::from_str(&exported.into_value_and_notices().0).map_err(failed)?;

    let kind_registry = state.kind_registry();
    let mut kinds = BTreeMap::new();
    let mut degrees = BTreeMap::new();
    for node in state.graph().nodes() {
        let kind = node.kind.raw.as_str();
        kinds
            .entry(kind.to_string())
            .or_insert_with(|| symbol_kind_from_entity(kind, kind_registry));
        degrees.insert(
            node.id.raw.to_string(),
            state.graph().degree(node.id.raw.as_str()).total(),
        );
    }
    Ok(json!({ "graph": graph, "kinds": kinds, "degrees": degrees }))
}

/// The Graph Protocol node of the entity the cursor at `position` of `uri` names
/// ([`entity_under_cursor`], ADR 0023 D5): what `specforge query <id> --depth 0` answers as its one
/// node. `None` where the cursor names no entity: a word in a string or a comment, a keyword, a field
/// name, a `use` path.
pub fn entity(state: &LspState, uri: &Url, position: Position) -> Result<Option<Value>> {
    let compiled = Compiled::new(state);
    let Some(id) = entity_under_cursor(&compiled, &state.key_of(uri), position) else {
        return Ok(None);
    };
    let answer = query::query(
        &state.view(),
        &QueryRequest {
            entity_id: id.as_str(),
            depth: Some(0),
            ..Default::default()
        },
    )
    .map_err(|refusal| failed(refusal.error.message))?;
    let mut document: Value =
        serde_json::from_str(&answer.into_value_and_notices().0).map_err(failed)?;
    Ok(document["nodes"].get_mut(0).map(Value::take))
}
