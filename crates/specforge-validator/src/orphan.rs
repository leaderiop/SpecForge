use specforge_common::{Diagnostic, Severity};
use specforge_graph::Graph;

const STRUCTURAL_KINDS: &[&str] = &["ref", "spec"];

pub fn detect_orphan_structural_nodes(graph: &Graph, diagnostics: &mut Vec<Diagnostic>) {
    for node in graph.nodes() {
        if !STRUCTURAL_KINDS.contains(&node.kind.raw.as_str()) {
            continue;
        }
        let incoming = graph.edges_to(&node.id.raw);
        if incoming.is_empty() {
            diagnostics.push(Diagnostic {
                code: "W012".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "unreferenced {} '{}' has no incoming edges",
                    node.kind.raw, node.id.raw
                ),
                span: Some(node.source_span.clone()),
                suggestion: None,
            });
        }
    }
}
