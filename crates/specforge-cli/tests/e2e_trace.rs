use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;
use std::collections::BTreeSet;

/// One direction of a trace: its (entity_id, depth) links, in order.
type Links = Vec<(String, u64)>;

/// `specforge trace <id>` in `dir`: (upstream, downstream).
fn trace(dir: &std::path::Path, id: &str) -> (Links, Links) {
    let output = specforge_cmd()
        .args(["trace", id])
        .arg("--path")
        .arg(dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["entity_id"], id);
    let links = |direction: &str| {
        parsed[direction]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                (
                    l["entity_id"].as_str().unwrap().to_string(),
                    l["depth"].as_u64().unwrap(),
                )
            })
            .collect()
    };
    (links("upstream"), links("downstream"))
}

fn links(list: &[(&str, u64)]) -> Links {
    list.iter().map(|(id, d)| (id.to_string(), *d)).collect()
}

/// The links as a set, after checking none repeats.
fn once_each(list: &Links) -> BTreeSet<(String, u64)> {
    let set: BTreeSet<_> = list.iter().cloned().collect();
    assert_eq!(set.len(), list.len(), "a node is visited twice: {list:?}");
    set
}

// --- Trace depth tests ---

#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace from entity shows upstream and downstream connections"
)]
fn trace_isolated_entity_has_empty_upstream_and_downstream() {
    let spec = format!(
        "{ISOLATED_SPEC}\nbehavior alpha \"A\" {{ contract \"first\" }}\nfeature beta \"B\" {{ problem \"p\" solution \"s\" behaviors [alpha] }}\n"
    );
    let dir = setup_project(&[("main.spec", &spec)]);

    // Unconnected: nothing either way.
    assert_eq!(trace(dir.path(), "isolated_node"), (links(&[]), links(&[])));
    // beta -behaviors-> alpha: each sees the other on its own side.
    assert_eq!(
        trace(dir.path(), "alpha"),
        (links(&[("beta", 1)]), links(&[]))
    );
    assert_eq!(
        trace(dir.path(), "beta"),
        (links(&[]), links(&[("alpha", 1)]))
    );
}

#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace shows full chain depth"
)]
fn trace_linear_chain_shows_correct_depths() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    // inv_deep -enforced_by-> beh_middle -features-> feat_root: the whole
    // chain, each hop at its depth.
    let (upstream, downstream) = trace(dir.path(), "inv_deep");
    assert_eq!(upstream, links(&[]));
    assert_eq!(downstream, links(&[("beh_middle", 1), ("feat_root", 2)]));
}

#[test]
fn trace_includes_edge_labels() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "inv_deep"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let upstream = parsed["upstream"].as_array().unwrap();

    for link in upstream {
        let label = link["edge_label"].as_str().unwrap();
        assert!(
            !label.is_empty(),
            "every TraceLink must have a non-empty edge_label"
        );
    }
}

#[specforge_test(
    invariant = "graph_traversal_integrity",
    verify = "traversal from any node visits every reachable node exactly once"
)]
fn trace_handles_cycles_without_hanging() {
    let dir = setup_project(&[("main.spec", CYCLE_SPEC)]);

    // Edges: cycle_a -> cycle_c, cycle_b -> cycle_a, cycle_c -> cycle_a.
    // From cycle_c the cycle is walked both ways, each other node once,
    // and the walk stops rather than coming back to cycle_c.
    let (upstream, downstream) = trace(dir.path(), "cycle_c");
    assert_eq!(
        once_each(&upstream),
        BTreeSet::from([("cycle_a".to_string(), 1), ("cycle_b".to_string(), 2)])
    );
    assert_eq!(
        once_each(&downstream),
        BTreeSet::from([("cycle_a".to_string(), 1)])
    );
}

#[specforge_test(
    invariant = "graph_traversal_integrity",
    verify = "traversal from any node visits every reachable node exactly once"
)]
fn trace_cycle_visits_each_node_once() {
    let dir = setup_project(&[("main.spec", CYCLE_SPEC)]);

    // Upstream of cycle_a: cycle_b and cycle_c point at it. Downstream:
    // cycle_a -> cycle_c, whose only edge leads back to cycle_a.
    let (upstream, downstream) = trace(dir.path(), "cycle_a");
    assert_eq!(
        once_each(&upstream),
        BTreeSet::from([("cycle_b".to_string(), 1), ("cycle_c".to_string(), 1)])
    );
    assert_eq!(
        once_each(&downstream),
        BTreeSet::from([("cycle_c".to_string(), 1)])
    );
}

#[specforge_test(
    invariant = "graph_traversal_integrity",
    verify = "identical graph inputs produce identical traversal results"
)]
fn trace_deterministic_output() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output1 = specforge_cmd()
        .args(["trace", "beh_middle"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    let output2 = specforge_cmd()
        .args(["trace", "beh_middle"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert_eq!(stdout1, stdout2, "trace output should be deterministic");
}

#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace from entity shows upstream and downstream connections"
)]
fn trace_root_entity_has_no_upstream() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    // inv_deep is a root: nothing points at it, but it reaches the chain.
    let (upstream, downstream) = trace(dir.path(), "inv_deep");
    assert_eq!(upstream, links(&[]), "inv_deep should have no upstream");
    assert_eq!(downstream, links(&[("beh_middle", 1), ("feat_root", 2)]));

    // beh_middle is in the middle: feat_root and inv_deep point at it, and
    // it points at feat_root.
    let (upstream, downstream) = trace(dir.path(), "beh_middle");
    assert_eq!(
        once_each(&upstream),
        BTreeSet::from([("feat_root".to_string(), 1), ("inv_deep".to_string(), 1)])
    );
    assert_eq!(downstream, links(&[("feat_root", 1)]));

    // typ_leaf has no references either way.
    assert_eq!(trace(dir.path(), "typ_leaf"), (links(&[]), links(&[])));
}

#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace from entity shows upstream and downstream connections"
)]
fn trace_leaf_entity_has_no_downstream() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature beta "B" { problem "p" solution "s" behaviors [alpha] }
"#,
    )]);

    // alpha has no outgoing edges (it's a leaf), but beta points to it
    let output = specforge_cmd()
        .args(["trace", "alpha"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["entity_id"], "alpha");
    assert_eq!(
        parsed["downstream"].as_array().unwrap().len(),
        0,
        "alpha should have no downstream"
    );
    assert!(
        !parsed["upstream"].as_array().unwrap().is_empty(),
        "alpha should have upstream (beta)"
    );
}

#[test]
fn trace_multi_kind_chain_preserves_entity_kind() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "inv_deep"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    assert_eq!(parsed["entity_kind"], "invariant");

    let upstream = parsed["upstream"].as_array().unwrap();
    for link in upstream {
        let kind = link["entity_kind"].as_str().unwrap();
        assert!(
            !kind.is_empty(),
            "every TraceLink should have a non-empty entity_kind"
        );
    }

    // beh_middle should have kind "behavior"
    if let Some(beh) = upstream.iter().find(|l| l["entity_id"] == "beh_middle") {
        assert_eq!(beh["entity_kind"], "behavior");
    }
}

#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "trace output includes schema version"
)]
fn trace_output_includes_schema_version() {
    let dir = setup_project(&[("main.spec", ISOLATED_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "isolated_node"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(
        parsed["schema_version"].is_string(),
        "trace output must include schema_version field"
    );
    assert!(!parsed["schema_version"].as_str().unwrap().is_empty());
}
