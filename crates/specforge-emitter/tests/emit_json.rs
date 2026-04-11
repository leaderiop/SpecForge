use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|s| s.to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

// B:serialize_json_graph — verify unit "output is valid JSON"
// B:serialize_json_graph — verify unit "empty graph produces valid JSON with empty nodes and edges arrays"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "output is valid JSON")]
fn empty_graph_produces_valid_json_with_empty_arrays() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["nodes"], serde_json::json!([]));
    assert_eq!(parsed["edges"], serde_json::json!([]));
    assert!(parsed["schema_version"].is_string(), "schema_version must be present");
}

// B:serialize_json_graph — verify unit "JSON output contains all nodes"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all nodes")]
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

// B:serialize_json_graph — verify unit "JSON output contains all edges"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all edges")]
fn json_contains_all_edges() {
    let mut graph = Graph::new();
    graph.add_node(node("feat_a", "feature", Some("Feature A")));
    graph.add_node(node("beh_b", "behavior", None));
    graph.add_edge(Edge {
        source: Sym::new("feat_a"),
        target: Sym::new("beh_b"),
        label: Sym::new("behaviors"),
    });

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let edges = parsed["edges"].as_array().expect("edges is array");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["source"], "feat_a");
    assert_eq!(edges[0]["target"], "beh_b");
    assert_eq!(edges[0]["label"], "behaviors");
}

// B:serialize_json_graph — verify unit "output includes schema_version field"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "output includes schema_version field")]
fn json_includes_schema_version() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let version = parsed["schema_version"].as_str().expect("schema_version is string");
    assert!(!version.is_empty());
}

// B:serialize_json_graph — verify unit "JSON output contains all nodes"
// (covers field serialization within nodes)
#[test]
#[specforge_test(behavior = "serialize_json_graph")]
fn json_includes_fields() {
    let mut fields = FieldMap::new();
    fields.push(Sym::new("contract"), FieldValue::String("The system MUST do X".to_string()));
    fields.push(Sym::new("status"), FieldValue::Identifier("done".to_string()));

    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: Sym::new("alpha") },
        kind: EntityKind { raw: Sym::new("behavior") },
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

// B:serialize_json_graph — verify unit "empty graph produces valid JSON with empty nodes and edges arrays"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "empty graph produces valid JSON with empty nodes and edges arrays")]
fn empty_graph_valid_json_with_empty_arrays() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert_eq!(parsed["nodes"], serde_json::json!([]));
    assert_eq!(parsed["edges"], serde_json::json!([]));
}

// B:serialize_json_graph — verify unit "schema is included even for empty graph"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "schema is included even for empty graph")]
fn schema_included_even_for_empty_graph() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let version = parsed["schema_version"].as_str().expect("schema_version is string");
    assert!(!version.is_empty());
}

// B:serialize_json_graph — verify integration "structural-only graph (zero extensions) produces valid Graph Protocol JSON with raw keywords in kind field"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "structural-only graph (zero extensions) produces valid Graph Protocol JSON with raw keywords in kind field")]
fn structural_only_graph_produces_valid_json_with_raw_keywords() {
    let mut graph = Graph::new();
    // Use non-standard keywords (simulating zero-extension / structural-only graph)
    graph.add_node(node("my_entity", "custom_kind", Some("Custom Entity")));
    graph.add_node(node("another", "unknown_type", None));

    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let nodes = parsed["nodes"].as_array().unwrap();
    // Nodes are sorted by ID: "another" < "my_entity"
    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"custom_kind"), "raw keyword preserved in kind field");
    assert!(kinds.contains(&"unknown_type"), "raw keyword preserved in kind field");
    assert!(parsed["schema_version"].is_string());
}

// B:serialize_json_graph — verify unit "JSON output contains all nodes"
// (covers source location metadata within nodes)
#[test]
#[specforge_test(behavior = "serialize_json_graph")]
fn json_includes_source_location() {
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: Sym::new("alpha") },
        kind: EntityKind { raw: Sym::new("behavior") },
        title: None,
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("behaviors/core.spec"),
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
