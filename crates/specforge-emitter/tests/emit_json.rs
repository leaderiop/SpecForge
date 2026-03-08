use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".to_string(),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: title.map(|s| s.to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

#[test]
fn empty_graph_produces_valid_json_with_empty_arrays() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["nodes"], serde_json::json!([]));
    assert_eq!(parsed["edges"], serde_json::json!([]));
    assert!(parsed["schema_version"].is_string(), "schema_version must be present");
}

#[test]
fn json_contains_all_nodes() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha Behavior")));
    graph.add_node(node("beta", "feature", Some("Beta Feature")));

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let nodes = parsed["nodes"].as_array().expect("nodes is array");
    assert_eq!(nodes.len(), 2);

    // Nodes sorted by ID for determinism
    assert_eq!(nodes[0]["id"], "alpha");
    assert_eq!(nodes[0]["kind"], "behavior");
    assert_eq!(nodes[0]["title"], "Alpha Behavior");
    assert_eq!(nodes[1]["id"], "beta");
    assert_eq!(nodes[1]["kind"], "feature");
    assert_eq!(nodes[1]["title"], "Beta Feature");
}

#[test]
fn json_contains_all_edges() {
    let mut graph = Graph::new();
    graph.add_node(node("feat_a", "feature", Some("Feature A")));
    graph.add_node(node("beh_b", "behavior", None));
    graph.add_edge(Edge {
        source: "feat_a".to_string(),
        target: "beh_b".to_string(),
        label: "behaviors".to_string(),
    });

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let edges = parsed["edges"].as_array().expect("edges is array");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["source"], "feat_a");
    assert_eq!(edges[0]["target"], "beh_b");
    assert_eq!(edges[0]["label"], "behaviors");
}

#[test]
fn json_includes_schema_version() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let version = parsed["schema_version"].as_str().expect("schema_version is string");
    assert!(!version.is_empty());
}

#[test]
fn json_includes_fields() {
    let mut fields = FieldMap::new();
    fields.push("contract".to_string(), FieldValue::String("The system MUST do X".to_string()));
    fields.push("status".to_string(), FieldValue::Identifier("done".to_string()));

    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: Some("Alpha".to_string()),
        fields,
        source_span: span(),
    });

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let node = &parsed["nodes"].as_array().unwrap()[0];
    let fields = &node["fields"];
    assert_eq!(fields["contract"], "The system MUST do X");
    assert_eq!(fields["status"], "done");
}

#[test]
fn json_includes_source_location() {
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: None,
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "behaviors/core.spec".to_string(),
            start_line: 10,
            start_col: 0,
            end_line: 25,
            end_col: 1,
        },
    });

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let node = &parsed["nodes"].as_array().unwrap()[0];
    assert_eq!(node["file"], "behaviors/core.spec");
    assert_eq!(node["line"], 10);
}
