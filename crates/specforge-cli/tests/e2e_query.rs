use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Query depth tests ---

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "depth 0 returns only root")]
fn query_depth_0_returns_only_root() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=0"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "depth 0 should return only the root entity");
    assert_eq!(nodes[0]["id"], "beh_middle");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "depth 0 still includes edges between surviving nodes")]
fn query_depth_0_edges_behavior() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=0"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    // With only one node, no edges can survive (both endpoints must be present)
    assert_eq!(edges.len(), 0, "depth 0 single node should have no edges");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "multiple kind filters")]
fn query_multiple_kind_filters() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=2", "--kind=behavior", "--kind=invariant"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    for kind in &kinds {
        assert!(
            *kind == "behavior" || *kind == "invariant",
            "all nodes should be behavior or invariant, got '{}'", kind
        );
    }
    assert!(nodes.len() >= 2, "should include at least root + inv_deep");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "kind filter no match returns root only")]
fn query_kind_filter_no_match_returns_root_only() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=2", "--kind=nonexistent_kind"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "only root should survive when kind filter matches nothing");
    assert_eq!(nodes[0]["id"], "beh_middle");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "kind filter prunes edges")]
fn query_kind_filter_prunes_edges() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    // Query with kind=behavior should prune edges to non-behavior nodes
    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=3", "--kind=behavior"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    let edges = parsed["edges"].as_array().unwrap();

    let node_ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();

    // Every edge endpoint must reference an existing node
    for edge in edges {
        let source = edge["source"].as_str().unwrap();
        let target = edge["target"].as_str().unwrap();
        assert!(
            node_ids.contains(&source),
            "edge source '{}' not in filtered nodes {:?}", source, node_ids
        );
        assert!(
            node_ids.contains(&target),
            "edge target '{}' not in filtered nodes {:?}", target, node_ids
        );
    }
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "isolated entity at depth 1")]
fn query_isolated_entity_depth_1() {
    let dir = setup_project(&[("main.spec", ISOLATED_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "isolated_node", "--depth=1"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "isolated node at depth 1 should return just that node");
    assert_eq!(nodes[0]["id"], "isolated_node");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "query handles cycles")]
fn query_handles_cycles() {
    let dir = setup_project(&[("main.spec", CYCLE_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "cycle_a", "--depth=10"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    // Must terminate (not hang) and succeed
    assert!(output.status.success(), "query on cyclic graph should terminate");
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    // Should contain all cycle nodes without duplicates
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    let unique: std::collections::HashSet<&&str> = ids.iter().collect();
    assert_eq!(ids.len(), unique.len(), "no duplicate nodes in query results");
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "large depth on small graph returns all")]
fn query_large_depth_on_small_graph() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=100"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    // Should get all connected nodes (beh_middle + feat_root + inv_deep, maybe typ_leaf if connected)
    assert!(nodes.len() >= 3, "depth 100 on 4-node graph should return all connected nodes, got {}", nodes.len());
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "output has schema version")]
fn query_output_has_schema_version() {
    let dir = setup_project(&[("main.spec", ISOLATED_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "isolated_node", "--depth=0"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(
        parsed["schema_version"].is_string(),
        "query output must include schema_version field"
    );
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "nodes have required fields")]
fn query_nodes_have_required_fields() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "beh_middle", "--depth=2"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    for node in nodes {
        assert!(node["id"].is_string(), "every node must have an 'id' field");
        assert!(node["kind"].is_string(), "every node must have a 'kind' field");
        assert!(!node["id"].as_str().unwrap().is_empty());
        assert!(!node["kind"].as_str().unwrap().is_empty());
    }
}
