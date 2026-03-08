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

fn build_graph() -> Graph {
    let mut graph = Graph::new();

    // Add nodes in non-alphabetical order to test determinism
    for (id, kind, title) in [
        ("zebra", "feature", "Zebra Feature"),
        ("alpha", "behavior", "Alpha Behavior"),
        ("middle", "invariant", "Middle Invariant"),
    ] {
        let mut fields = FieldMap::new();
        fields.push("contract".to_string(), FieldValue::String(format!("Contract for {}", id)));
        graph.add_node(Node {
            id: EntityId { raw: id.to_string() },
            kind: EntityKind { raw: kind.to_string() },
            title: Some(title.to_string()),
            fields,
            source_span: span(),
        });
    }

    graph.add_edge(Edge {
        source: "zebra".to_string(),
        target: "alpha".to_string(),
        label: "behaviors".to_string(),
    });
    graph.add_edge(Edge {
        source: "zebra".to_string(),
        target: "middle".to_string(),
        label: "invariants".to_string(),
    });

    graph
}

#[test]
fn same_input_produces_identical_json_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_json(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "JSON output must be identical across runs");
    }
}

#[test]
fn same_input_produces_identical_dot_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_dot(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "DOT output must be identical across runs");
    }
}

#[test]
fn same_input_produces_identical_brief_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_brief(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "Brief output must be identical across runs");
    }
}

#[test]
fn same_input_produces_identical_context_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_context(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "Context output must be identical across runs");
    }
}

#[test]
fn json_output_contains_no_timestamps() {
    let json = specforge_emitter::emit_json(&build_graph());
    let lower = json.to_lowercase();
    assert!(!lower.contains("timestamp"), "output must not contain timestamp");
    assert!(!lower.contains("generated_at"), "output must not contain generated_at");
}
