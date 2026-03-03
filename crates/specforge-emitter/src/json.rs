use crate::types::GeneratedFile;
use serde::Serialize;
use specforge_common::CompilerConfig;
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;

#[derive(Debug, Serialize)]
struct JsonGraph {
    metadata: JsonMetadata,
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Debug, Serialize)]
struct JsonMetadata {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_prefix: Option<String>,
    version: String,
}

#[derive(Debug, Serialize)]
struct JsonNode {
    id: String,
    kind: String,
    title: Option<String>,
    file: String,
}

#[derive(Debug, Serialize)]
struct JsonEdge {
    source: String,
    target: String,
    edge_type: String,
    field_name: String,
}

/// Render the spec graph as a JSON export file.
///
/// Nodes are sorted by raw ID, edges sorted by (source, target, edge_type)
/// for deterministic output (BEH-SF-070).
pub fn render_json(graph: &SpecGraph, _files: &[SpecFile], config: &CompilerConfig) -> GeneratedFile {
    let metadata = JsonMetadata {
        name: config.name.clone(),
        namespace: config.namespace.clone(),
        display_prefix: config.display_prefix.clone(),
        version: config.version.to_string(),
    };

    let mut nodes: Vec<JsonNode> = graph
        .nodes()
        .map(|n| JsonNode {
            id: n.id.raw().to_string(),
            kind: n.kind.keyword().to_string(),
            title: n.title.clone(),
            file: n.file.clone(),
        })
        .collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    let mut edges: Vec<JsonEdge> = graph
        .edges()
        .map(|(src, tgt, edge)| JsonEdge {
            source: src.id.raw().to_string(),
            target: tgt.id.raw().to_string(),
            edge_type: edge.edge_type.label().to_string(),
            field_name: edge.field_name.clone(),
        })
        .collect();
    edges.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then(a.target.cmp(&b.target))
            .then(a.edge_type.cmp(&b.edge_type))
    });

    let json_graph = JsonGraph {
        metadata,
        nodes,
        edges,
    };

    let content = serde_json::to_string_pretty(&json_graph).expect("JSON serialization failed");

    GeneratedFile {
        path: "graph.json".to_string(),
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{CompilerConfig, EntityId, EntityKind, EdgeType, SourceSpan};
    use specforge_graph::{GraphEdge, GraphNode, SpecGraph};

    fn test_config() -> CompilerConfig {
        CompilerConfig::core_only("test-project")
    }

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
        graph.add_node(make_node("INV-TS-1", EntityKind::Invariant, "Data Integrity"));
        graph.add_node(make_node("BEH-TS-1", EntityKind::Behavior, "Validate Input"));
        graph.add_node(make_node("FEAT-TS-1", EntityKind::Feature, "Validation"));
        graph.add_edge(
            "BEH-TS-1",
            "INV-TS-1",
            GraphEdge {
                edge_type: EdgeType::References,
                field_name: "invariants".to_string(),
            },
        );
        graph.add_edge(
            "FEAT-TS-1",
            "BEH-TS-1",
            GraphEdge {
                edge_type: EdgeType::Implements,
                field_name: "behaviors".to_string(),
            },
        );
        graph
    }

    #[test]
    fn json_output_is_valid_json() {
        let graph = make_graph();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn json_contains_all_nodes() {
        let graph = make_graph();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let nodes = parsed["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), graph.node_count());
    }

    #[test]
    fn json_contains_all_edges() {
        let graph = make_graph();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        let edges = parsed["edges"].as_array().unwrap();
        assert_eq!(edges.len(), graph.edge_count());
    }

    #[test]
    fn json_deterministic() {
        let graph = make_graph();
        let config = test_config();
        let a = render_json(&graph, &[], &config);
        let b = render_json(&graph, &[], &config);
        assert_eq!(a.content, b.content);
    }

    #[test]
    fn json_includes_metadata() {
        let graph = make_graph();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(parsed["metadata"]["name"], "test-project");
        assert_eq!(parsed["metadata"]["version"], "1.0");
    }

    #[test]
    fn json_empty_graph() {
        let graph = SpecGraph::new();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        assert_eq!(parsed["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["edges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn json_snapshot() {
        let graph = make_graph();
        let result = render_json(&graph, &[], &test_config());
        let parsed: serde_json::Value = serde_json::from_str(&result.content).unwrap();
        insta::assert_json_snapshot!(parsed);
    }
}
