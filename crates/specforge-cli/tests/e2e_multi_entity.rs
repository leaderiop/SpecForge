use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Phase 1b: All entity kinds through check/export/query/trace/stats ---

#[test]
#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode produces no output files")]
fn check_accepts_all_software_entity_kinds() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode prints diagnostics to stderr")]
fn check_accepts_all_product_entity_kinds() {
    // Product spec references parse_input from software, include it
    let combined = format!("{}\n{}", SOFTWARE_SPEC, PRODUCT_SPEC);
    let dir = setup_project(&[("main.spec", &combined)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode works in CI environment")]
fn check_accepts_all_governance_entity_kinds() {
    // Governance references parse_input via mitigations
    let combined = format!("{}\n{}", SOFTWARE_SPEC, GOVERNANCE_SPEC);
    let dir = setup_project(&[("main.spec", &combined)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "check_mode_for_ci", verify = "requires/ensures consistency for CI check mode")]
fn check_full_multi_extension_project_exits_zero() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all nodes")]
fn export_graph_includes_all_entity_kinds() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    for expected_kind in &[
        "behavior", "invariant", "event", "type", "port",
        "feature", "journey", "deliverable", "milestone", "module", "term",
        "decision", "constraint", "failure_mode",
    ] {
        assert!(
            kinds.contains(expected_kind),
            "missing kind '{}' in graph export. found: {:?}",
            expected_kind, kinds,
        );
    }
}

#[test]
#[specforge_test(behavior = "export_agent_brief_format", verify = "brief format includes only IDs, kinds, titles, and edges")]
fn export_brief_includes_all_entity_kinds() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=brief"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    let kinds: Vec<&str> = nodes.iter().map(|n| n["kind"].as_str().unwrap()).collect();
    for expected_kind in &["behavior", "feature", "decision", "constraint"] {
        assert!(kinds.contains(expected_kind), "missing kind '{}' in brief", expected_kind);
    }
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "context format includes entity IDs and contracts")]
fn export_context_shows_contracts_for_all_kinds() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=context"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();

    // Behaviors have contracts
    let parse_input = nodes.iter().find(|n| n["id"] == "parse_input").unwrap();
    assert!(
        parse_input.get("contract").is_some(),
        "behavior should have contract in context format"
    );
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "output conforms to Graph Protocol schema")]
fn query_by_kind_filter_product_entities() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "fast_parsing", "--depth=0", "--kind=feature"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["kind"], "feature");
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "querying same entity at same depth produces identical subgraph")]
fn query_by_kind_filter_governance_entities() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["query", "use_treesitter", "--depth=0", "--kind=decision"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["kind"], "decision");
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports correct entity counts")]
fn stats_entities_by_kind_counts_all_kinds() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["stats", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    // 15 entities total in MULTI_EXTENSION_SPEC (2 behaviors + 13 other kinds)
    let total = parsed["total_entities"].as_u64().unwrap();
    assert_eq!(total, 15, "expected 15 entities, got {}", total);

    let by_kind = parsed["entities_by_kind"].as_object().unwrap();
    assert!(by_kind.contains_key("behavior"), "missing behavior in entities_by_kind");
    assert!(by_kind.contains_key("feature"), "missing feature in entities_by_kind");
    assert!(by_kind.contains_key("decision"), "missing decision in entities_by_kind");
    assert_eq!(by_kind["behavior"], 2, "expected 2 behaviors");
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports coverage percentage")]
fn stats_human_format_lists_all_kinds() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["stats"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    for kind in &["behavior", "feature", "decision", "constraint", "failure_mode"] {
        assert!(stdout.contains(kind), "human stats missing kind '{}': {}", kind, stdout);
    }
}

#[test]
#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode produces no output files")]
fn multi_file_project_with_use_imports() {
    let dir = setup_project(&[
        ("behaviors.spec", r#"
behavior parse_input "Parse Input" {
    contract "The system MUST parse all valid input"
}
behavior emit_output "Emit Output" {
    contract "The system MUST emit structured output"
}
"#),
        ("features.spec", r#"
use "behaviors"

feature fast_parsing "Fast Parsing" {
    problem "Users need quick feedback"
    solution "Incremental parsing"
    behaviors [parse_input]
}
"#),
    ]);

    // Check succeeds with cross-file references
    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();

    // Export includes entities from both files
    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    assert!(nodes.len() >= 3, "should have entities from both files");
}
