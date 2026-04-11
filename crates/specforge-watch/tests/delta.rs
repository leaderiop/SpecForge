use specforge_graph::{Edge, Graph, Node};
use specforge_common::{SourceSpan, Sym};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use specforge_watch::{compute_graph_delta, GraphDelta};
use specforge_test_macros::test as spec;

fn make_node(id: &str, kind: &str, file: &str, line: usize) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 0,
        },
    }
}

// ── compute_graph_delta: added nodes ──────────────────────────

#[spec(behavior = "compute_graph_delta", verify = "added nodes appear in delta")]
#[test]
fn added_nodes_appear_in_delta() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("foo", "behavior", "a.spec", 1));

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.added_nodes.len(), 1);
    assert_eq!(delta.added_nodes[0].id, "foo");
    assert_eq!(delta.added_nodes[0].kind, "behavior");
    assert_eq!(delta.removed_nodes.len(), 0);
    assert_eq!(delta.modified_nodes.len(), 0);
}

// ── compute_graph_delta: removed nodes ────────────────────────

#[spec(behavior = "compute_graph_delta", verify = "removed nodes appear in delta")]
#[test]
fn removed_nodes_appear_in_delta() {
    let mut old = Graph::new();
    old.add_node(make_node("bar", "feature", "b.spec", 5));
    let new = Graph::new();

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.removed_nodes.len(), 1);
    assert_eq!(delta.removed_nodes[0].id, "bar");
    assert_eq!(delta.removed_nodes[0].kind, "feature");
    assert_eq!(delta.added_nodes.len(), 0);
}

// ── compute_graph_delta: modified nodes ───────────────────────

#[spec(behavior = "compute_graph_delta", verify = "modified nodes list changed fields")]
#[test]
fn modified_nodes_list_changed_fields() {
    let mut old = Graph::new();
    let mut old_node = make_node("baz", "behavior", "c.spec", 1);
    old_node.fields.push(Sym::new("status"), FieldValue::String("draft".to_string()));
    old.add_node(old_node);

    let mut new = Graph::new();
    let mut new_node = make_node("baz", "behavior", "c.spec", 1);
    new_node.fields.push(Sym::new("status"), FieldValue::String("done".to_string()));
    new.add_node(new_node);

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.added_nodes.len(), 0);
    assert_eq!(delta.removed_nodes.len(), 0);
    assert_eq!(delta.modified_nodes.len(), 1);
    assert_eq!(delta.modified_nodes[0].id, "baz");
    assert!(delta.modified_nodes[0].changed_fields.contains(&"status".to_string()));
}

// ── compute_graph_delta: edges ────────────────────────────────

#[spec(behavior = "compute_graph_delta", verify = "added and removed edges appear in delta")]
#[test]
fn added_and_removed_edges_appear_in_delta() {
    let mut old = Graph::new();
    old.add_node(make_node("a", "behavior", "a.spec", 1));
    old.add_node(make_node("b", "feature", "a.spec", 5));
    old.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("features"),
    });

    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));
    new.add_node(make_node("c", "invariant", "a.spec", 10));
    new.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("c"),
        label: Sym::new("invariants"),
    });

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.removed_edges.len(), 1);
    assert_eq!(delta.removed_edges[0].source, "a");
    assert_eq!(delta.removed_edges[0].target, "b");

    assert_eq!(delta.added_edges.len(), 1);
    assert_eq!(delta.added_edges[0].source, "a");
    assert_eq!(delta.added_edges[0].target, "c");
}

// ── compute_graph_delta: affected files ───────────────────────

#[spec(behavior = "compute_graph_delta", verify = "affected files listed in delta")]
#[test]
fn affected_files_listed_in_delta() {
    let mut old = Graph::new();
    old.add_node(make_node("x", "behavior", "file1.spec", 1));

    let mut new = Graph::new();
    new.add_node(make_node("y", "feature", "file2.spec", 1));

    let delta = compute_graph_delta(&old, &new);

    assert!(delta.affected_files.contains(&"file1.spec".to_string()));
    assert!(delta.affected_files.contains(&"file2.spec".to_string()));
}

// ── compute_graph_delta: deterministic sort ───────────────────

#[spec(behavior = "compute_graph_delta")]
#[test]
fn delta_arrays_sorted_by_entity_id() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("z_last", "behavior", "a.spec", 1));
    new.add_node(make_node("a_first", "feature", "a.spec", 5));
    new.add_node(make_node("m_middle", "invariant", "a.spec", 10));

    let delta = compute_graph_delta(&old, &new);

    let ids: Vec<&str> = delta.added_nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["a_first", "m_middle", "z_last"]);
}

// ── compute_graph_delta: no changes ───────────────────────────

#[spec(behavior = "compute_graph_delta")]
#[test]
fn identical_graphs_produce_empty_delta() {
    let mut old = Graph::new();
    old.add_node(make_node("same", "behavior", "a.spec", 1));

    let mut new = Graph::new();
    new.add_node(make_node("same", "behavior", "a.spec", 1));

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.added_nodes.len(), 0);
    assert_eq!(delta.removed_nodes.len(), 0);
    assert_eq!(delta.modified_nodes.len(), 0);
    assert_eq!(delta.added_edges.len(), 0);
    assert_eq!(delta.removed_edges.len(), 0);
    assert_eq!(delta.affected_files.len(), 0);
}

// ── compute_graph_delta: delta_include_values ─────────────────

#[spec(behavior = "compute_graph_delta", verify = "delta_include_values=true populates old_value and new_value")]
#[test]
fn delta_include_values_populates_old_and_new_value() {
    use specforge_watch::DeltaConfig;

    let mut old = Graph::new();
    let mut old_node = make_node("x", "behavior", "a.spec", 1);
    old_node.fields.push(Sym::new("status"), FieldValue::String("draft".to_string()));
    old.add_node(old_node);

    let mut new = Graph::new();
    let mut new_node = make_node("x", "behavior", "a.spec", 1);
    new_node.fields.push(Sym::new("status"), FieldValue::String("done".to_string()));
    new.add_node(new_node);

    let config = DeltaConfig { include_values: true };
    let delta = specforge_watch::compute_graph_delta_with_config(&old, &new, &config);

    assert_eq!(delta.modified_nodes.len(), 1);
    assert!(delta.modified_nodes[0].old_value.is_some());
    assert!(delta.modified_nodes[0].new_value.is_some());
}

#[spec(behavior = "compute_graph_delta", verify = "delta_include_values=false omits old_value and new_value")]
#[test]
fn delta_default_config_omits_values() {
    let mut old = Graph::new();
    let mut old_node = make_node("x", "behavior", "a.spec", 1);
    old_node.fields.push(Sym::new("status"), FieldValue::String("draft".to_string()));
    old.add_node(old_node);

    let mut new = Graph::new();
    let mut new_node = make_node("x", "behavior", "a.spec", 1);
    new_node.fields.push(Sym::new("status"), FieldValue::String("done".to_string()));
    new.add_node(new_node);

    let delta = compute_graph_delta(&old, &new);

    assert_eq!(delta.modified_nodes.len(), 1);
    assert!(delta.modified_nodes[0].old_value.is_none());
    assert!(delta.modified_nodes[0].new_value.is_none());
}

// ── validate_delta_correctness ────────────────────────────────

#[spec(behavior = "validate_delta_correctness", verify = "delta applied to old graph equals new graph")]
#[test]
fn validate_delta_passes_for_correct_delta() {
    use specforge_watch::validate_delta_correctness;

    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));

    let delta = compute_graph_delta(&old, &new);
    let result = validate_delta_correctness(&old, &new, &delta);
    assert!(result.is_ok());
}

#[spec(behavior = "validate_delta_correctness", verify = "successful validation emits delta_validation_passed with node and edge counts")]
#[test]
fn successful_validation_returns_node_and_edge_counts() {
    use specforge_watch::validate_delta_correctness;

    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));
    new.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("features"),
    });

    let delta = compute_graph_delta(&old, &new);
    let result = validate_delta_correctness(&old, &new, &delta).unwrap();

    assert_eq!(result.node_count, 2);
    assert_eq!(result.edge_count, 1);
}

#[spec(behavior = "validate_delta_correctness", verify = "discrepancy triggers debug assertion with descriptive message")]
#[test]
fn validate_delta_fails_for_wrong_node_count() {
    use specforge_watch::{validate_delta_correctness, NodeChange};

    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));

    // Fabricate a bad delta — only one added node when there should be two
    let bad_delta = GraphDelta {
        added_nodes: vec![NodeChange {
            id: "a".to_string(),
            kind: "behavior".to_string(),
            file: Some("a.spec".to_string()),
            line: Some(1),
        }],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec!["a.spec".to_string()],
    };

    let result = validate_delta_correctness(&old, &new, &bad_delta);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("node count"), "expected node count mismatch, got: {}", msg);
}

#[spec(behavior = "validate_delta_correctness", verify = "discrepancy triggers debug assertion with descriptive message")]
#[test]
fn validate_delta_fails_for_wrong_edge_count() {
    use specforge_watch::validate_delta_correctness;

    let mut old = Graph::new();
    old.add_node(make_node("a", "behavior", "a.spec", 1));
    old.add_node(make_node("b", "feature", "a.spec", 5));

    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));
    new.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("features"),
    });

    // Fabricate delta that claims no edge changes
    let bad_delta = GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    };

    let result = validate_delta_correctness(&old, &new, &bad_delta);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("edge count"), "expected edge count mismatch, got: {}", msg);
}

#[spec(behavior = "validate_delta_correctness", verify = "discrepancy triggers debug assertion with descriptive message")]
#[test]
fn validate_delta_fails_when_added_node_missing_from_new_graph() {
    use specforge_watch::{validate_delta_correctness, NodeChange};

    let old = Graph::new();
    let new = Graph::new(); // empty — but delta claims node was added

    let bad_delta = GraphDelta {
        added_nodes: vec![NodeChange {
            id: "phantom".to_string(),
            kind: "behavior".to_string(),
            file: None,
            line: None,
        }],
        removed_nodes: vec![NodeChange {
            id: "phantom_removed".to_string(),
            kind: "behavior".to_string(),
            file: None,
            line: None,
        }],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    };

    // node count: 0 + 1 - 1 = 0, matches new.node_count() = 0, so count check passes
    // but "phantom" doesn't exist in new graph
    let result = validate_delta_correctness(&old, &new, &bad_delta);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("phantom"), "expected phantom node error, got: {}", msg);
}

#[spec(behavior = "validate_delta_correctness", verify = "check disabled in release builds")]
#[test]
fn validate_delta_disabled_skips_checks() {
    use specforge_watch::validate_delta_correctness_if_enabled;

    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(make_node("a", "behavior", "a.spec", 1));
    new.add_node(make_node("b", "feature", "a.spec", 5));

    // Fabricate a bad delta — should fail if validation is enabled
    let bad_delta = GraphDelta {
        added_nodes: vec![],
        removed_nodes: vec![],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    };

    // Enabled: should detect the mismatch
    let enabled_result = validate_delta_correctness_if_enabled(&old, &new, &bad_delta, true);
    assert!(enabled_result.is_err(), "validation should fail when enabled");

    // Disabled: should skip and return Ok
    let disabled_result = validate_delta_correctness_if_enabled(&old, &new, &bad_delta, false);
    assert!(disabled_result.is_ok(), "validation should be skipped when disabled");
    let counts = disabled_result.unwrap();
    assert_eq!(counts.node_count, 2);
    assert_eq!(counts.edge_count, 0);
}

#[spec(behavior = "validate_delta_correctness", verify = "discrepancy triggers debug assertion with descriptive message")]
#[test]
fn validate_delta_fails_when_removed_node_still_in_new_graph() {
    use specforge_watch::{validate_delta_correctness, NodeChange};

    let mut old = Graph::new();
    old.add_node(make_node("still_here", "behavior", "a.spec", 1));
    old.add_node(make_node("will_go", "behavior", "a.spec", 5));

    let mut new = Graph::new();
    new.add_node(make_node("still_here", "behavior", "a.spec", 1));
    new.add_node(make_node("newcomer", "behavior", "a.spec", 10));

    // Delta claims "still_here" was removed (wrong) and "newcomer" was added (right)
    // Count: 2 + 1 - 1 = 2 = new.node_count(), so count check passes
    let bad_delta = GraphDelta {
        added_nodes: vec![NodeChange {
            id: "newcomer".to_string(),
            kind: "behavior".to_string(),
            file: None,
            line: None,
        }],
        removed_nodes: vec![NodeChange {
            id: "still_here".to_string(),
            kind: "behavior".to_string(),
            file: None,
            line: None,
        }],
        modified_nodes: vec![],
        added_edges: vec![],
        removed_edges: vec![],
        affected_files: vec![],
    };

    // count: 1 + 1 - 1 = 1 = new.node_count(), but "still_here" shouldn't be in new
    let result = validate_delta_correctness(&old, &new, &bad_delta);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("still present"), "expected 'still present' error, got: {}", msg);
}
