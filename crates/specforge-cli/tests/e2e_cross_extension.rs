use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Phase 2a: Cross-extension references, I004 soft resolution, did-you-mean ---

#[test]
#[specforge_test(behavior = "link_entity_references", verify = "reference list IDs create graph edges")]
fn cross_kind_references_resolve() {
    let dir = setup_project(&[("main.spec", CROSS_REF_SPEC)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "link_entity_references", verify = "unresolvable reference produces E003")]
fn unresolved_cross_ref_produces_e003() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent_behavior] }
"#)]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let parsed = parse_json_stdout(&output);
    let diagnostics = parsed.as_array().unwrap();
    assert!(
        diagnostics.iter().any(|d| d["code"] == "E003"),
        "should have E003 for unresolved reference: {:?}", diagnostics,
    );
}

#[test]
#[specforge_test(behavior = "link_entity_references", verify = "reference list IDs create graph edges")]
fn governance_references_software_entities() {
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

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "link_entity_references", verify = "reference list IDs create graph edges")]
fn product_references_software_entities() {
    let dir = setup_project(&[("main.spec", r#"
behavior parse_input "P" { contract "must parse" }
feature fast_parsing "F" {
    problem "need speed"
    solution "be fast"
    behaviors [parse_input]
}
"#)]);

    specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "diagnostics serialized as JSON array to stdout")]
fn check_json_shows_cross_extension_errors() {
    let dir = setup_project(&[("main.spec", r#"
feature gamma "G" { behaviors [nonexistent] }
decision use_rust "D" {
    status accepted
    context "need performance"
    decision_text "use Rust"
    consequences "fast"
}
"#)]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    let parsed = parse_json_stdout(&output);
    let diagnostics = parsed.as_array().unwrap();
    assert!(
        diagnostics.iter().any(|d| d["code"] == "E003"),
        "should produce E003 for nonexistent ref in cross-kind context"
    );
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all edges")]
fn export_includes_cross_kind_edges() {
    let dir = setup_project(&[("main.spec", CROSS_REF_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    let edges = parsed["edges"].as_array().unwrap();

    // Should have edges crossing kind boundaries: feature->behavior, invariant->behavior, etc.
    let edge_pairs: Vec<(&str, &str)> = edges.iter().map(|e| {
        (e["source"].as_str().unwrap(), e["target"].as_str().unwrap())
    }).collect();

    // graph_validation -> validate_graph (behaviors), graph_validation -> resolve_refs (behaviors)
    assert!(
        edge_pairs.iter().any(|(s, t)| *s == "graph_validation" && (*t == "validate_graph" || *t == "resolve_refs")),
        "should have cross-kind edges from feature to behavior: {:?}", edge_pairs,
    );
}

#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "trace from entity shows upstream and downstream connections")]
fn trace_crosses_extension_boundaries() {
    let dir = setup_project(&[("main.spec", CROSS_REF_SPEC)]);

    let output = specforge_cmd()
        .args(["trace", "unresolved_ref"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["entity_id"], "unresolved_ref");

    // unresolved_ref (failure_mode) -> resolve_refs (behavior) via mitigations
    let downstream = parsed["downstream"].as_array().unwrap();
    let downstream_ids: Vec<&str> = downstream.iter().map(|l| l["entity_id"].as_str().unwrap()).collect();
    assert!(
        downstream_ids.contains(&"resolve_refs"),
        "trace should cross from governance entity to software entity: {:?}", downstream_ids,
    );
}

#[test]
#[specforge_test(behavior = "link_entity_references", verify = "close match triggers did-you-mean suggestion")]
fn did_you_mean_works_across_entity_kinds() {
    let dir = setup_project(&[("main.spec", r#"
behavior validate_graph "V" { contract "must validate" }
feature graph_validation "G" {
    problem "must validate"
    solution "validation"
    behaviors [validat_graph]
}
"#)]);

    let output = specforge_cmd()
        .args(["check"])
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean") && stderr.contains("validate_graph"),
        "should suggest correct entity across kinds: {}", stderr,
    );
}
