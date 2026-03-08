use specforge_graph::Graph;

fn resolve_scope(graph: &Graph, scope: &str) -> Result<Graph, String> {
    graph.subgraph(scope).ok_or_else(|| {
        format!("E001: unresolved scope entity '{}' — entity not found in graph", scope)
    })
}

pub fn emit_json_scoped(graph: &Graph, scope: &str) -> Result<String, String> {
    let sub = resolve_scope(graph, scope)?;
    Ok(crate::json::emit_json(&sub))
}

pub fn emit_context_scoped(graph: &Graph, scope: &str) -> Result<String, String> {
    let sub = resolve_scope(graph, scope)?;
    Ok(crate::context::emit_context(&sub))
}
