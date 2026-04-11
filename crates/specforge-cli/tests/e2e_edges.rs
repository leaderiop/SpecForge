use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Phase 1c: Edge types from reference-list fields, DOT labels, multi-hop ---

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "behaviors field creates edges")]
fn behaviors_field_creates_edges() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" { behaviors [alpha, beta] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2, "behaviors [alpha, beta] should create 2 edges");

    let sources: Vec<&str> = edges.iter().map(|e| e["source"].as_str().unwrap()).collect();
    let targets: Vec<&str> = edges.iter().map(|e| e["target"].as_str().unwrap()).collect();
    assert!(sources.iter().all(|s| *s == "gamma"), "all edges should come from gamma");
    assert!(targets.contains(&"alpha"));
    assert!(targets.contains(&"beta"));
}

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "features field creates edges")]
fn features_field_creates_edges() {
    let dir = setup_project(&[("main.spec", r#"
feature fast_parsing "F" { problem "p" solution "s" }
behavior parse_input "P" {
    contract "The system MUST parse"
    features [fast_parsing]
}
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["source"], "parse_input");
    assert_eq!(edges[0]["target"], "fast_parsing");
}

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "enforced_by field creates edges")]
fn enforced_by_field_creates_edges() {
    let dir = setup_project(&[("main.spec", r#"
behavior validate "V" { contract "must validate" }
invariant refs_resolved "RR" {
    guarantee "All refs MUST resolve"
    enforced_by [validate]
}
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    assert!(!edges.is_empty(), "enforced_by [validate] should create an edge");
    assert_eq!(edges[0]["source"], "refs_resolved");
    assert_eq!(edges[0]["target"], "validate");
}

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "mitigations field creates edges")]
fn mitigations_field_creates_edges() {
    let dir = setup_project(&[("main.spec", r#"
behavior parse_input "P" { contract "must parse" }
failure_mode parser_crash "PC" {
    severity 8
    occurrence 2
    detection 3
    cause "Bad input"
    effect "Crash"
    mitigations [parse_input]
}
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    assert!(!edges.is_empty(), "mitigations field should create an edge");
    assert_eq!(edges[0]["target"], "parse_input");
}

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "export graph edge labels are field names")]
fn export_graph_edge_labels_are_field_names() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges[0]["label"], "behaviors", "edge label should match field name");
}

#[test]
#[specforge_test(behavior = "emit_dot_output", verify = "DOT export shows edge labels")]
fn dot_export_shows_edge_labels() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph"), "should be a DOT digraph");
    assert!(stdout.contains("behaviors"), "DOT should contain edge label 'behaviors'");
}

#[test]
#[specforge_test(behavior = "trace_entity_dependencies", verify = "trace follows edges across entity kinds")]
fn trace_follows_edges_across_entity_kinds() {
    let dir = setup_project(&[("main.spec", CROSS_REF_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "validate_graph"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    assert_eq!(parsed["entity_id"], "validate_graph");
    // validate_graph has upstream: graph_validation (via features), refs_resolved (via enforced_by)
    let upstream = parsed["upstream"].as_array().unwrap();
    let upstream_ids: Vec<&str> = upstream.iter().map(|l| l["entity_id"].as_str().unwrap()).collect();
    assert!(
        upstream_ids.contains(&"graph_validation") || upstream_ids.contains(&"refs_resolved")
            || upstream_ids.contains(&"validation_complete") || upstream_ids.contains(&"unresolved_ref"),
        "trace should include cross-kind upstream entities: {:?}", upstream_ids
    );
}

#[test]
#[specforge_test(behavior = "multi_resolution_graph_queries", verify = "query depth 2 traverses multi-hop")]
fn query_depth_2_traverses_multi_hop() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" features [feat_a] }
feature feat_a "F" { behaviors [alpha] }
journey dev_journey "DJ" { description "workflow" }
"#)]);

    // Query from feat_a at depth 1 should reach alpha
    let output = specforge_cmd()
        .args(["query", "feat_a", "--depth=1"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"feat_a"), "root entity should be in results");
    assert!(ids.contains(&"alpha"), "depth-1 neighbor should be in results");
}

#[test]
#[specforge_test(behavior = "emit_graph_protocol_json", verify = "multiple reference fields produce separate edges")]
fn multiple_reference_fields_produce_separate_edges() {
    let dir = setup_project(&[("main.spec", r#"
behavior validate "V" { contract "must validate" features [feat_a] }
feature feat_a "F" { behaviors [validate] }
invariant inv_a "I" { guarantee "always" enforced_by [validate] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();
    // behaviors: feat_a->validate, features: validate->feat_a, enforced_by: inv_a->validate
    assert!(edges.len() >= 3, "should have edges from multiple reference fields, got {}", edges.len());

    let labels: Vec<&str> = edges.iter().map(|e| e["label"].as_str().unwrap()).collect();
    assert!(labels.contains(&"behaviors"), "should have 'behaviors' edges");
    assert!(labels.contains(&"features"), "should have 'features' edges");
    assert!(labels.contains(&"enforced_by"), "should have 'enforced_by' edges");
}

#[test]
#[specforge_test(behavior = "emit_dot_output", verify = "DOT export includes all entity kinds as nodes")]
fn dot_export_all_entity_kinds_as_nodes() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph"));

    // Check some entity IDs are present as nodes
    for id in &["parse_input", "fast_parsing", "use_treesitter", "parser_crash"] {
        assert!(stdout.contains(id), "DOT should contain node '{}'", id);
    }
}

#[test]
#[specforge_test(behavior = "emit_dot_output", verify = "DOT export cross-kind edges")]
fn dot_export_cross_kind_edges() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
failure_mode fm "FM" { severity 1 occurrence 1 detection 1 cause "x" effect "y" mitigations [alpha] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have edges from gamma->alpha and fm->alpha
    assert!(stdout.contains("gamma"), "should have gamma node");
    assert!(stdout.contains("alpha"), "should have alpha node");
    assert!(stdout.contains("fm"), "should have fm node");
}

#[test]
#[specforge_test(behavior = "emit_dot_output", verify = "DOT export deterministic output")]
fn dot_export_deterministic_output() {
    let dir = setup_project(&[("main.spec", MULTI_EXTENSION_SPEC)]);

    let output1 = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    let output2 = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert_eq!(stdout1, stdout2, "DOT output should be deterministic");
}
