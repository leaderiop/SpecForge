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

fn node_with_contract(id: &str, kind: &str, title: &str, contract: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push("contract".to_string(), FieldValue::String(contract.to_string()));
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: Some(title.to_string()),
        fields,
        source_span: span(),
    }
}

#[test]
fn brief_includes_only_ids_kinds_titles_and_edges() {
    let mut graph = Graph::new();
    graph.add_node(node_with_contract(
        "alpha", "behavior", "Alpha",
        "The system MUST do alpha things with lots of verbose prose.",
    ));
    graph.add_node(node("beta", "feature", Some("Beta")));
    graph.add_edge(Edge {
        source: "beta".to_string(),
        target: "alpha".to_string(),
        label: "behaviors".to_string(),
    });

    let json = specforge_emitter::emit_brief(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    // Brief format: only id, kind, title — no contract or other fields
    assert_eq!(nodes[0]["id"], "alpha");
    assert_eq!(nodes[0]["kind"], "behavior");
    assert_eq!(nodes[0]["title"], "Alpha");
    assert!(nodes[0].get("contract").is_none(), "brief must not include contract");
    assert!(nodes[0].get("fields").is_none(), "brief must not include fields");

    // Has edges
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);

    // Has schema_version
    assert!(parsed["schema_version"].is_string());
}

#[test]
fn brief_is_smaller_than_full_json() {
    let mut graph = Graph::new();
    graph.add_node(node_with_contract(
        "alpha", "behavior", "Alpha",
        "The system MUST do alpha things with lots of verbose prose that makes the output larger.",
    ));

    let brief = specforge_emitter::emit_brief(&graph);
    let full = specforge_emitter::emit_json(&graph);
    assert!(brief.len() < full.len(), "brief ({}) should be smaller than full ({})", brief.len(), full.len());
}

#[test]
fn brief_schema_version_matches_graph_format() {
    let mut graph = Graph::new();
    graph.add_node(node("x", "behavior", Some("X")));

    let brief: serde_json::Value = serde_json::from_str(&specforge_emitter::emit_brief(&graph)).unwrap();
    let full: serde_json::Value = serde_json::from_str(&specforge_emitter::emit_json(&graph)).unwrap();
    assert_eq!(brief["schema_version"], full["schema_version"],
        "brief and graph formats must use same schema_version");
}
