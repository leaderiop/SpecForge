use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
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

#[test]
fn emit_graph_includes_all_fields_and_metadata() {
    let mut fields = FieldMap::new();
    fields.push("contract".to_string(), FieldValue::String("Must do X".to_string()));
    fields.push("status".to_string(), FieldValue::Identifier("done".to_string()));

    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: Some("Alpha".to_string()),
        fields,
        source_span: span(),
    });

    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    let node = &parsed["nodes"].as_array().unwrap()[0];
    // Full fidelity: includes fields, file, line
    assert_eq!(node["fields"]["contract"], "Must do X");
    assert_eq!(node["fields"]["status"], "done");
    assert_eq!(node["file"], "test.spec");
    assert_eq!(node["line"], 1);
    assert!(parsed["schema_version"].is_string());
}

#[test]
fn emit_graph_equals_emit_json() {
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: Some("Alpha".to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let json_output = specforge_emitter::emit_json(&graph);
    let graph_output = specforge_emitter::emit_graph(&graph);
    assert_eq!(json_output, graph_output);
}
