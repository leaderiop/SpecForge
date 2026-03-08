use specforge_graph::Graph;

pub fn query(graph: &Graph, entity_id: &str, depth: usize, kind_filter: &[&str]) -> Result<String, String> {
    let sub = graph.subgraph_depth(entity_id, depth).ok_or_else(|| {
        format!("E001: unresolved entity '{}' — not found in graph", entity_id)
    })?;

    // Apply kind filter: keep root + nodes matching any specified kind
    let filtered = if kind_filter.is_empty() {
        sub
    } else {
        let mut filtered = Graph::new();
        for node in sub.nodes() {
            if node.id.raw == entity_id || kind_filter.contains(&node.kind.raw.as_str()) {
                filtered.add_node(node.clone());
            }
        }
        // Only keep edges where both endpoints survive filtering
        for edge in sub.edges() {
            if filtered.node(&edge.source).is_some() && filtered.node(&edge.target).is_some() {
                filtered.add_edge(edge.clone());
            }
        }
        filtered
    };

    Ok(crate::json::emit_json(&filtered))
}
