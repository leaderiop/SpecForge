use crate::types::GeneratedFile;
use specforge_common::EntityKind;
use specforge_graph::SpecGraph;
use std::fmt::Write;

/// Map entity kind to a DOT node shape.
fn dot_shape(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Invariant | EntityKind::TypeDef | EntityKind::Port => "box",
        EntityKind::Behavior | EntityKind::Event => "ellipse",
        EntityKind::Feature | EntityKind::Capability => "hexagon",
        EntityKind::Deliverable | EntityKind::Library => "folder",
        EntityKind::Decision | EntityKind::Constraint | EntityKind::FailureMode => "note",
        EntityKind::Ref => "diamond",
        EntityKind::Spec | EntityKind::Glossary | EntityKind::Roadmap => "tab",
    }
}

/// Escape a string for use in DOT labels.
fn dot_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Render the spec graph as a Graphviz DOT file.
///
/// Nodes and edges are sorted by ID for deterministic output.
pub fn render_dot(graph: &SpecGraph) -> GeneratedFile {
    let mut out = String::new();
    writeln!(out, "digraph specforge {{").unwrap();
    writeln!(out, "  rankdir=LR;").unwrap();
    writeln!(out, "  node [shape=box, fontname=\"Helvetica\"];").unwrap();

    // Collect and sort nodes
    let mut nodes: Vec<_> = graph.nodes().collect();
    nodes.sort_by_key(|n| n.id.raw().to_string());

    for node in &nodes {
        let id = dot_escape(node.id.raw());
        let title = node
            .title
            .as_deref()
            .map(|t| format!("\\n{}", dot_escape(t)))
            .unwrap_or_default();
        let shape = dot_shape(node.kind);
        writeln!(
            out,
            "  \"{id}\" [label=\"{id}{title}\" shape={shape}];",
        )
        .unwrap();
    }

    // Collect and sort edges
    let mut edges: Vec<_> = graph
        .edges()
        .map(|(src, tgt, edge)| {
            (
                src.id.raw().to_string(),
                tgt.id.raw().to_string(),
                edge.edge_type.label().to_string(),
            )
        })
        .collect();
    edges.sort();

    for (src, tgt, label) in &edges {
        let src = dot_escape(src);
        let tgt = dot_escape(tgt);
        let label = dot_escape(label);
        writeln!(out, "  \"{src}\" -> \"{tgt}\" [label=\"{label}\"];").unwrap();
    }

    writeln!(out, "}}").unwrap();

    GeneratedFile {
        path: "graph.dot".to_string(),
        content: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{EdgeType, EntityId, EntityKind, SourceSpan};
    use specforge_graph::{GraphEdge, GraphNode, SpecGraph};

    fn make_node(id: &str, kind: EntityKind, title: &str) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(title.to_string()),
            file: "test.spec".to_string(),
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    fn make_graph() -> SpecGraph {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("data_integrity", EntityKind::Invariant, "Data Integrity"));
        graph.add_node(make_node("validate_input", EntityKind::Behavior, "Validate Input"));
        graph.add_edge(
            "validate_input",
            "data_integrity",
            GraphEdge {
                edge_type: EdgeType::References,
                field_name: "invariants".to_string(),
            },
        );
        graph
    }

    #[test]
    fn dot_valid_syntax() {
        let graph = make_graph();
        let result = render_dot(&graph);
        assert!(result.content.starts_with("digraph specforge {"));
        assert!(result.content.trim_end().ends_with('}'));
    }

    #[test]
    fn dot_contains_all_nodes() {
        let graph = make_graph();
        let result = render_dot(&graph);
        assert!(result.content.contains("\"validate_input\""));
        assert!(result.content.contains("\"data_integrity\""));
    }

    #[test]
    fn dot_edges_labeled() {
        let graph = make_graph();
        let result = render_dot(&graph);
        assert!(result.content.contains("[label=\"references\"]"));
    }

    #[test]
    fn dot_deterministic() {
        let graph = make_graph();
        let a = render_dot(&graph);
        let b = render_dot(&graph);
        assert_eq!(a.content, b.content);
    }

    #[test]
    fn dot_snapshot() {
        let graph = make_graph();
        let result = render_dot(&graph);
        insta::assert_snapshot!(result.content);
    }
}
