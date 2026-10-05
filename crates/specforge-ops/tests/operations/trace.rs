use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_ops::trace::{TraceExpectations, TraceLinkStatus};
use specforge_parser::{EntityId, EntityKind, FieldMap};
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

fn node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("Title {}", id)),
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    }
}

/// a -> b -> c (linear chain)
fn build_chain() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "invariant"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    graph.add_edge(Edge {
        source: "b".into(),
        target: "c".into(),
        label: "invariants".into(),
    });
    graph
}

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace from entity shows upstream and downstream connections"
)]
fn trace_shows_upstream_and_downstream() {
    let graph = build_chain();
    let trace = specforge_ops::trace::trace(&graph, "b").unwrap();

    assert!(
        trace.upstream.iter().any(|l| l.entity_id == "a"),
        "upstream should include a"
    );
    assert!(
        trace.downstream.iter().any(|l| l.entity_id == "c"),
        "downstream should include c"
    );
}

// B:compute_traceability_chain — verify unit "trace shows full chain depth"
#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace shows full chain depth"
)]
fn trace_shows_full_chain_depth() {
    let mut graph = build_chain();
    graph.add_node(node("d", "event"));
    graph.add_edge(Edge {
        source: "c".into(),
        target: "d".into(),
        label: "produces".into(),
    });

    let trace = specforge_ops::trace::trace(&graph, "a").unwrap();
    // a is root, so no upstream, downstream = b, c, d
    assert!(trace.upstream.is_empty());
    let ids: Vec<&str> = trace
        .downstream
        .iter()
        .map(|l| l.entity_id.as_str())
        .collect();
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
    assert!(ids.contains(&"d"));
}

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (error case: nonexistent entity returns error)
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_nonexistent_entity_returns_error() {
    let graph = build_chain();
    let result = specforge_ops::trace::trace(&graph, "nonexistent");
    assert!(result.is_err());
}

// B:serialize_traceability_data — verify unit "output conforms to Graph Protocol schema"
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "output conforms to Graph Protocol schema"
)]
fn trace_serializes_to_json() {
    let graph = build_chain();
    let trace = specforge_ops::trace::trace(&graph, "b").unwrap();
    let json = specforge_ops::trace::serialize_trace(&trace).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["entity_id"], "b");
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
    assert!(parsed["schema_version"].is_string());
}

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (edge case: leaf has upstream only)
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_on_leaf_has_upstream_only() {
    let graph = build_chain();
    let trace = specforge_ops::trace::trace(&graph, "c").unwrap();

    assert!(!trace.upstream.is_empty(), "leaf should have upstream");
    assert!(
        trace.downstream.is_empty(),
        "leaf should have no downstream"
    );
}

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (edge case: root has downstream only)
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_on_root_has_downstream_only() {
    let graph = build_chain();
    let trace = specforge_ops::trace::trace(&graph, "a").unwrap();

    assert!(trace.upstream.is_empty(), "root should have no upstream");
    assert!(!trace.downstream.is_empty(), "root should have downstream");
}

// B:serialize_traceability_data — verify unit "full trace covers all root entities across registered edge types"
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "full trace covers all root entities across registered edge types"
)]
fn trace_all_covers_all_root_entities() {
    let graph = build_chain();
    let traces = specforge_ops::trace::trace_all(&graph);
    // All 3 entities should have a trace
    assert_eq!(traces.len(), 3);
    let ids: Vec<&str> = traces.iter().map(|t| t.entity_id.as_str()).collect();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
}

// B:serialize_traceability_data — verify unit "output conforms to Graph Protocol schema"
#[specforge_test(behavior = "serialize_traceability_data")]
fn trace_all_serializes_as_json_array() {
    let graph = build_chain();
    let traces = specforge_ops::trace::trace_all(&graph);
    let json = specforge_ops::trace::serialize_trace_all(&traces).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
    assert_eq!(parsed["traces"].as_array().unwrap().len(), 3);
}

fn expectations() -> TraceExpectations {
    let (fields, kinds) = crate::trace_support::registries();
    TraceExpectations::from_registries(&fields, &kinds)
}

/// The missing links of a chain as (from, edge_label, expected_kind).
fn missing(chain: &specforge_ops::trace::TraceChain) -> Vec<(&str, &str, &str)> {
    chain
        .missing
        .iter()
        .map(|m| {
            assert_eq!(m.status, TraceLinkStatus::Missing);
            (
                m.from.as_str(),
                m.edge_label.as_str(),
                m.expected_kind.as_str(),
            )
        })
        .collect()
}

// B:compute_traceability_chain — verify unit "missing link in chain is flagged"
#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "missing link in chain is flagged"
)]
fn trace_missing_link_flagged() {
    // a -behaviors-> b: b's `features` link is declared from the feature's
    // side; b declares no invariant, which its kind expects.
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });

    let trace =
        specforge_ops::trace::trace_with_expectations(&graph, "b", &expectations()).unwrap();
    assert_eq!(missing(&trace), vec![("b", "invariants", "invariant")]);
    let gap = &trace.missing[0];
    assert_eq!(gap.from_kind, "behavior");
    assert_eq!(gap.edge_type.as_deref(), Some("behavior_invariants"));
    assert_eq!(gap.depth, 1);
    assert!(!gap.required);
    // The resolved links are unchanged and say so.
    assert_eq!(trace.upstream.len(), 1);
    assert_eq!(trace.upstream[0].status, TraceLinkStatus::Resolved);

    // Declaring the invariant closes the gap.
    graph.add_node(node("i", "invariant"));
    graph.add_edge(Edge {
        source: "b".into(),
        target: "i".into(),
        label: "invariants".into(),
    });
    let trace =
        specforge_ops::trace::trace_with_expectations(&graph, "b", &expectations()).unwrap();
    assert!(trace.missing.is_empty(), "{:?}", trace.missing);

    // A feature with no behavior misses one; a behavior nothing links
    // misses both.
    graph.add_node(node("lonely", "feature"));
    graph.add_node(node("orphan", "behavior"));
    let lonely =
        specforge_ops::trace::trace_with_expectations(&graph, "lonely", &expectations()).unwrap();
    assert_eq!(missing(&lonely), vec![("lonely", "behaviors", "behavior")]);
    let orphan =
        specforge_ops::trace::trace_with_expectations(&graph, "orphan", &expectations()).unwrap();
    assert_eq!(
        missing(&orphan),
        vec![
            ("orphan", "features", "feature"),
            ("orphan", "invariants", "invariant"),
        ]
    );
}

// B:compute_traceability_chain — a missing link is an expected edge that is
// absent, not a broken reference: a dangling edge is E003 in check.
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_dangling_edge_is_not_a_missing_link() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    // b declares an invariant that does not exist.
    graph.add_edge(Edge {
        source: "b".into(),
        target: "phantom".into(),
        label: "invariants".into(),
    });

    let trace =
        specforge_ops::trace::trace_with_expectations(&graph, "b", &expectations()).unwrap();
    assert!(trace.missing.is_empty(), "{:?}", trace.missing);
    assert!(
        !trace.downstream.iter().any(|l| l.entity_id == "phantom"),
        "an undeclared entity is not part of the chain"
    );
    assert_eq!(
        specforge_ops::trace::detect_trace_gaps(&graph),
        vec!["dangling edge target 'phantom' in edge b -> phantom (invariants)".to_string()]
    );
}

// B:compute_traceability_chain — the expected edges come from the
// registries alone.
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_expectations_come_from_the_registries() {
    let expected = expectations();
    let labels = |kind: &str| -> Vec<String> {
        expected
            .for_kind(kind)
            .iter()
            .map(|e| e.label.clone())
            .collect()
    };
    // ports: its kind isn't loaded. depends_on: a self-relation.
    // satisfies: another extension's. contract: not a reference.
    assert_eq!(labels("behavior"), vec!["features", "invariants"]);
    assert_eq!(labels("feature"), vec!["behaviors"]);
    assert!(labels("invariant").is_empty());
    // features is also declared by the feature's `behaviors`, which names
    // no inverse itself.
    let features = &expected.for_kind("behavior")[0];
    assert_eq!(features.inverse_labels, vec!["behaviors"]);
    assert_eq!(features.target_kind, "feature");

    // A required reference is expected wherever it comes from.
    let (mut fields, kinds) = crate::trace_support::registries();
    let mut parent =
        crate::trace_support::reference("behavior", "parent", "behavior", "@t/formal", None);
    parent.declared.required = true;
    fields.register(parent);
    let expected = TraceExpectations::from_registries(&fields, &kinds);
    let parent = expected
        .for_kind("behavior")
        .iter()
        .find(|e| e.label == "parent")
        .expect("required reference is expected");
    assert!(parent.required);

    // No extensions, no expectations.
    assert!(
        TraceExpectations::from_registries(
            &specforge_registry::FieldRegistry::new(),
            &specforge_registry::KindRegistry::new()
        )
        .is_empty()
    );
}

// B:compute_traceability_chain — human output marks each missing link.
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_human_output_marks_missing_links() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    let trace =
        specforge_ops::trace::trace_with_expectations(&graph, "b", &expectations()).unwrap();
    assert_eq!(
        specforge_ops::trace::render_trace_human(&trace),
        concat!(
            "b [behavior]\n",
            "  upstream:\n",
            "    <-behaviors- a [feature] (depth 1)\n",
            "  downstream:\n",
            "    (none)\n",
            "  missing:\n",
            "    MISSING b -invariants-> [invariant] (depth 1)\n",
        )
    );
}
