use specforge_common::Diagnostic;
use specforge_graph::Graph;

const STRUCTURAL_KINDS: &[&str] = &["ref", "spec"];

pub fn detect_orphan_structural_nodes(graph: &Graph, diagnostics: &mut Vec<Diagnostic>) {
    for node in graph.nodes() {
        if !STRUCTURAL_KINDS.contains(&node.kind.raw.as_str()) {
            continue;
        }
        let incoming = graph.edges_to(node.id.raw.as_str());
        if incoming.is_empty() {
            diagnostics.push(
                Diagnostic::warning(
                    "W012",
                    format!(
                        "unreferenced {} '{}' has no incoming edges",
                        node.kind.raw, node.id.raw
                    ),
                )
                .with_span(node.source_span.clone()),
            );
        }
    }
}
