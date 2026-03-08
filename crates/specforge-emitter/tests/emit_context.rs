use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".to_string(),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn rich_node() -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        "contract".to_string(),
        FieldValue::String("The system MUST do X".to_string()),
    );
    fields.push(
        "description".to_string(),
        FieldValue::String("A very long verbose prose field that agents don't need".to_string()),
    );
    fields.push(
        "status".to_string(),
        FieldValue::Identifier("done".to_string()),
    );
    fields.push(
        "verify".to_string(),
        FieldValue::VerifyList(vec![
            VerifyStatement {
                kind: "unit".to_string(),
                description: "it works".to_string(),
            },
        ]),
    );
    Node {
        id: EntityId { raw: "alpha".to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: Some("Alpha Behavior".to_string()),
        fields,
        source_span: span(),
    }
}

#[test]
fn context_includes_contracts_and_verify() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());

    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let node = &parsed["nodes"].as_array().unwrap()[0];

    // Context includes contract
    assert_eq!(node["contract"], "The system MUST do X");

    // Context includes verify declarations
    let verify = node["verify"].as_array().unwrap();
    assert_eq!(verify.len(), 1);
    assert_eq!(verify[0]["kind"], "unit");

    // Context includes status
    assert_eq!(node["status"], "done");
}

#[test]
fn context_omits_verbose_prose_fields() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());

    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let node = &parsed["nodes"].as_array().unwrap()[0];

    // Context omits description (verbose prose)
    assert!(
        node.get("description").is_none() || node["description"].is_null(),
        "context must omit description"
    );
}

#[test]
fn context_includes_edges_and_schema_version() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());
    let mut node_b = Node {
        id: EntityId { raw: "beta".to_string() },
        kind: EntityKind { raw: "feature".to_string() },
        title: Some("Beta".to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    };
    node_b.fields.push(
        "contract".to_string(),
        FieldValue::String("Feature contract".to_string()),
    );
    graph.add_node(node_b);
    graph.add_edge(Edge {
        source: "beta".to_string(),
        target: "alpha".to_string(),
        label: "behaviors".to_string(),
    });

    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(parsed["schema_version"].is_string());
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 1);
}

#[test]
fn context_is_smaller_than_full_json() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());

    let context = specforge_emitter::emit_context(&graph);
    let full = specforge_emitter::emit_json(&graph);
    assert!(
        context.len() < full.len(),
        "context ({}) should be smaller than full ({})",
        context.len(),
        full.len()
    );
}
