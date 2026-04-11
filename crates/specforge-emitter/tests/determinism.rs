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

fn build_graph() -> Graph {
    let mut graph = Graph::new();

    // Add nodes in non-alphabetical order to test determinism
    for (id, kind, title) in [
        ("zebra", "feature", "Zebra Feature"),
        ("alpha", "behavior", "Alpha Behavior"),
        ("middle", "invariant", "Middle Invariant"),
    ] {
        let mut fields = FieldMap::new();
        fields.push(Sym::new("contract"), FieldValue::String(format!("Contract for {}", id)));
        graph.add_node(Node {
            id: EntityId { raw: Sym::new(id) },
            kind: EntityKind { raw: Sym::new(kind) },
            title: Some(title.to_string()),
            fields,
            source_span: span(),
        });
    }

    graph.add_edge(Edge {
        source: Sym::new("zebra"),
        target: Sym::new("alpha"),
        label: Sym::new("behaviors"),
    });
    graph.add_edge(Edge {
        source: Sym::new("zebra"),
        target: Sym::new("middle"),
        label: Sym::new("invariants"),
    });

    graph
}

// B:deterministic_output — verify property "same input produces identical output across runs"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "same input produces identical output across runs")]
fn same_input_produces_identical_json_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_json(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "JSON output must be identical across runs");
    }
}

// B:deterministic_output — verify property "same input produces identical output across runs"
// (DOT format)
#[test]
#[specforge_test(behavior = "deterministic_output")]
fn same_input_produces_identical_dot_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_dot(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "DOT output must be identical across runs");
    }
}

// B:deterministic_output — verify property "same input produces identical output across runs"
// (brief format)
#[test]
#[specforge_test(behavior = "deterministic_output")]
fn same_input_produces_identical_brief_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_brief(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "Brief output must be identical across runs");
    }
}

// B:deterministic_output — verify property "same input produces identical output across runs"
// (context format)
#[test]
#[specforge_test(behavior = "deterministic_output")]
fn same_input_produces_identical_context_across_runs() {
    let outputs: Vec<String> = (0..5).map(|_| specforge_emitter::emit_context(&build_graph())).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "Context output must be identical across runs");
    }
}

// B:deterministic_output — verify unit "entity ordering is independent of hashmap iteration"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "entity ordering is independent of hashmap iteration")]
fn json_nodes_sorted_by_id() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    // Nodes added as zebra, alpha, middle — output must be sorted: alpha, middle, zebra
    assert_eq!(ids, vec!["alpha", "middle", "zebra"]);
}

// B:deterministic_output — verify unit "edge ordering is deterministic"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "edge ordering is deterministic")]
fn json_edges_sorted_deterministically() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let edges: Vec<(&str, &str)> = parsed["edges"].as_array().unwrap()
        .iter().map(|e| (e["source"].as_str().unwrap(), e["target"].as_str().unwrap())).collect();
    // Edges should be sorted by (source, target)
    let mut sorted = edges.clone();
    sorted.sort();
    assert_eq!(edges, sorted, "edges must be deterministically sorted");
}

// B:deterministic_output — verify unit "output contains no timestamps or non-deterministic values"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "output contains no timestamps or non-deterministic values")]
fn json_output_contains_no_timestamps() {
    let json = specforge_emitter::emit_json(&build_graph());
    let lower = json.to_lowercase();
    assert!(!lower.contains("timestamp"), "output must not contain timestamp");
    assert!(!lower.contains("generated_at"), "output must not contain generated_at");
}

// B:deterministic_output — verify unit "file emission order is independent of filesystem readdir order"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "file emission order is independent of filesystem readdir order")]
fn file_emission_order_independent_of_filesystem() {
    // Build two graphs with nodes from different "files" added in different orders
    let mut graph1 = Graph::new();
    let mut graph2 = Graph::new();

    let files = ["z_file.spec", "a_file.spec", "m_file.spec"];
    let ids = ["node_z", "node_a", "node_m"];

    // Graph1: add in order z, a, m
    for i in 0..3 {
        let mut fields = FieldMap::new();
        fields.push(Sym::new("contract"), FieldValue::String(format!("Contract {}", ids[i])));
        graph1.add_node(Node {
            id: EntityId { raw: Sym::new(ids[i]) },
            kind: EntityKind { raw: Sym::new("behavior") },
            title: Some(format!("Title {}", ids[i])),
            fields,
            source_span: SourceSpan {
                file: Sym::new(files[i]),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            },
        });
    }

    // Graph2: add in reverse order m, a, z
    for i in (0..3).rev() {
        let mut fields = FieldMap::new();
        fields.push(Sym::new("contract"), FieldValue::String(format!("Contract {}", ids[i])));
        graph2.add_node(Node {
            id: EntityId { raw: Sym::new(ids[i]) },
            kind: EntityKind { raw: Sym::new("behavior") },
            title: Some(format!("Title {}", ids[i])),
            fields,
            source_span: SourceSpan {
                file: Sym::new(files[i]),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 0,
            },
        });
    }

    let json1 = specforge_emitter::emit_json(&graph1);
    let json2 = specforge_emitter::emit_json(&graph2);
    assert_eq!(json1, json2, "output must be identical regardless of node insertion order (simulating different readdir orders)");
}

// B:deterministic_output — verify property "stats output is deterministic"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "stats output is deterministic")]
fn stats_output_is_deterministic() {
    let outputs: Vec<_> = (0..5).map(|_| {
        let stats = specforge_emitter::compute_stats(&build_graph());
        format!("{:?}", stats)
    }).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "stats output must be identical across runs");
    }
}

// B:deterministic_output — verify property "trace output is deterministic"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "trace output is deterministic")]
fn trace_output_is_deterministic() {
    let graph = build_graph();
    let outputs: Vec<_> = (0..5).map(|_| {
        specforge_emitter::serialize_trace(&specforge_emitter::trace(&graph, "alpha").unwrap())
    }).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "trace output must be identical across runs");
    }
}

// B:deterministic_output — verify property "scoped emit is deterministic"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "scoped emit is deterministic")]
fn scoped_json_output_is_deterministic() {
    let graph = build_graph();
    let outputs: Vec<_> = (0..5).map(|_| {
        specforge_emitter::emit_json_scoped(&graph, "zebra").unwrap()
    }).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "scoped JSON output must be identical across runs");
    }
}

// B:deterministic_output — verify property "scoped context emit is deterministic"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "scoped context emit is deterministic")]
fn scoped_context_output_is_deterministic() {
    let graph = build_graph();
    let outputs: Vec<_> = (0..5).map(|_| {
        specforge_emitter::emit_context_scoped(&graph, "zebra").unwrap()
    }).collect();
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "scoped context output must be identical across runs");
    }
}
