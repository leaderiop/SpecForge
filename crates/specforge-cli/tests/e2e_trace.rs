use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Trace depth tests ---

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "isolated entity has empty upstream and downstream")]
fn trace_isolated_entity_has_empty_upstream_and_downstream() {
    let dir = setup_project(&[("main.spec", ISOLATED_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "isolated_node"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    assert_eq!(parsed["entity_id"], "isolated_node");
    assert_eq!(parsed["upstream"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["downstream"].as_array().unwrap().len(), 0);
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "linear chain shows correct depths")]
fn trace_linear_chain_shows_correct_depths() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    // Trace from inv_deep: downstream should include beh_middle (depth 1)
    // because inv_deep → beh_middle via enforced_by (inv_deep is source)
    let output = specforge_cmd()
        .args(["trace", "inv_deep"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    assert_eq!(parsed["entity_id"], "inv_deep");
    let downstream = parsed["downstream"].as_array().unwrap();
    assert!(!downstream.is_empty(), "inv_deep should have downstream links via enforced_by");

    // beh_middle should be at depth 1
    let beh = downstream.iter().find(|l| l["entity_id"] == "beh_middle");
    assert!(beh.is_some(), "beh_middle should be in downstream");
    assert_eq!(beh.unwrap()["depth"], 1);
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "trace includes edge labels")]
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
        assert!(!label.is_empty(), "every TraceLink must have a non-empty edge_label");
    }
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "trace handles cycles without hanging")]
fn trace_handles_cycles_without_hanging() {
    let dir = setup_project(&[("main.spec", CYCLE_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "cycle_a"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    // Must terminate (not hang) and succeed
    assert!(output.status.success(), "trace on cyclic graph should terminate successfully");
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["entity_id"], "cycle_a");
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "cycle visits each node once")]
fn trace_cycle_visits_each_node_once() {
    let dir = setup_project(&[("main.spec", CYCLE_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "cycle_a"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    // Check no duplicates within each direction (upstream and downstream have separate visited sets)
    let upstream_ids: Vec<&str> = parsed["upstream"].as_array().unwrap()
        .iter().map(|l| l["entity_id"].as_str().unwrap()).collect();
    let downstream_ids: Vec<&str> = parsed["downstream"].as_array().unwrap()
        .iter().map(|l| l["entity_id"].as_str().unwrap()).collect();

    let unique_up: std::collections::HashSet<&&str> = upstream_ids.iter().collect();
    let unique_down: std::collections::HashSet<&&str> = downstream_ids.iter().collect();
    assert_eq!(upstream_ids.len(), unique_up.len(), "upstream BFS should not produce duplicates: {:?}", upstream_ids);
    assert_eq!(downstream_ids.len(), unique_down.len(), "downstream BFS should not produce duplicates: {:?}", downstream_ids);
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "deterministic output")]
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

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "root entity has no upstream")]
fn trace_root_entity_has_no_upstream() {
    let dir = setup_project(&[("main.spec", DEEP_CHAIN_SPEC)]);

    // typ_leaf has no references pointing to it (no incoming edges)
    let output = specforge_cmd()
        .args(["trace", "typ_leaf"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["entity_id"], "typ_leaf");
    assert_eq!(parsed["upstream"].as_array().unwrap().len(), 0, "typ_leaf should have no upstream");
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "leaf entity has no downstream")]
fn trace_leaf_entity_has_no_downstream() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature beta "B" { problem "p" solution "s" behaviors [alpha] }
"#)]);

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
    assert_eq!(parsed["downstream"].as_array().unwrap().len(), 0, "alpha should have no downstream");
    assert!(!parsed["upstream"].as_array().unwrap().is_empty(), "alpha should have upstream (beta)");
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "multi kind chain preserves entity kind")]
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

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "trace output includes schema version")]
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
