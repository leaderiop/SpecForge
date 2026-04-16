use specforge_graph::Graph;

use crate::error::EmitterError;

fn resolve_scope(graph: &Graph, scope: &str) -> Result<Graph, EmitterError> {
    graph.subgraph(scope).ok_or_else(|| {
        EmitterError::EntityNotFound(format!("E001: unresolved scope entity '{}' — entity not found in graph", scope))
    })
}

pub fn emit_json_scoped(graph: &Graph, scope: &str) -> Result<String, EmitterError> {
    let sub = resolve_scope(graph, scope)?;
    Ok(crate::json::emit_json(&sub))
}

pub fn emit_context_scoped(graph: &Graph, scope: &str) -> Result<String, EmitterError> {
    let sub = resolve_scope(graph, scope)?;
    Ok(crate::context::emit_context(&sub))
}
