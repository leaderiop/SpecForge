use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
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

fn rich_node() -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        Sym::new("contract"),
        FieldValue::String("The system MUST do X".to_string()),
    );
    fields.push(
        Sym::new("description"),
        FieldValue::String("A very long verbose prose field that agents don't need".to_string()),
    );
    fields.push(
        Sym::new("status"),
        FieldValue::Identifier("done".to_string()),
    );
    fields.push(
        Sym::new("verify"),
        FieldValue::VerifyList(vec![
            VerifyStatement {
                kind: "unit".to_string(),
                description: "it works".to_string(),
            },
        ]),
    );
    Node {
        id: EntityId { raw: Sym::new("alpha") },
        kind: EntityKind { raw: Sym::new("behavior") },
        title: Some("Alpha Behavior".to_string()),
        fields,
        source_span: span(),
    }
}

// B:export_agent_context_format — verify unit "context format includes entity IDs and contracts"
#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "context format includes entity IDs and contracts")]
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

// B:export_agent_context_format — verify unit "context format omits verbose prose fields"
#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "context format omits verbose prose fields")]
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

// B:export_agent_context_format — verify unit "output includes schema_version field"
// B:export_agent_context_format — verify unit "output conforms to Graph Protocol schema"
#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "output includes schema_version field")]
fn context_includes_edges_and_schema_version() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());
    let mut node_b = Node {
        id: EntityId { raw: Sym::new("beta") },
        kind: EntityKind { raw: Sym::new("feature") },
        title: Some("Beta".to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    };
    node_b.fields.push(
        Sym::new("contract"),
        FieldValue::String("Feature contract".to_string()),
    );
    graph.add_node(node_b);
    graph.add_edge(Edge {
        source: Sym::new("beta"),
        target: Sym::new("alpha"),
        label: Sym::new("behaviors"),
    });

    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(parsed["schema_version"].is_string());
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 1);
}

// B:export_agent_context_format — verify unit "output conforms to Graph Protocol schema"
#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "output conforms to Graph Protocol schema")]
fn context_conforms_to_graph_protocol_schema() {
    let mut graph = Graph::new();
    graph.add_node(rich_node());

    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    // Graph Protocol requires: schema_version, nodes array, edges array
    assert!(parsed["schema_version"].is_string(), "must have schema_version");
    assert!(parsed["nodes"].is_array(), "must have nodes array");
    assert!(parsed["edges"].is_array(), "must have edges array");

    // Each node must have id and kind
    for node in parsed["nodes"].as_array().unwrap() {
        assert!(node["id"].is_string(), "each node must have id");
        assert!(node["kind"].is_string(), "each node must have kind");
    }
}

// B:export_agent_context_format — verify unit "context format omits verbose prose fields"
// (demonstrates token optimization by size comparison)
#[test]
#[specforge_test(behavior = "export_agent_context_format")]
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
