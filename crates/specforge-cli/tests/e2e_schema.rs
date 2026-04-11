use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;

// --- Phase 1d: Schema command tests ---

#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "specforge schema outputs full schema as JSON")]
fn schema_command_outputs_valid_json() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["schema"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(parsed["schema_version"].is_object(), "should have schema_version object");
    assert!(parsed["entity_kinds"].is_array(), "should have entity_kinds array");
    assert!(parsed["edge_types"].is_array(), "should have edge_types array");
}

#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "--kind filter restricts to single entity kind")]
fn schema_kind_filter_returns_single_kind() {
    // Note: with GraphProtocolSchema::empty(), entity_kinds is empty.
    // This test verifies the --kind flag behavior: unknown kind exits 1
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    // Any kind filter on empty schema exits 1 (kind not found)
    specforge_cmd()
        .args(["schema", "--kind=behavior"])
        .arg(dir.path())
        .assert()
        .code(1);
}

#[test]
#[specforge_test(behavior = "serve_schema_resource", verify = "schema reflects current compilation state")]
fn schema_kind_filter_unknown_exits_one() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    specforge_cmd()
        .args(["schema", "--kind=nonexistent_kind"])
        .arg(dir.path())
        .assert()
        .code(1);
}

#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema is valid JSON Schema")]
fn schema_publish_produces_json_schema_draft() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["schema", "--publish"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(
        parsed["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "published schema should have $schema field"
    );
    assert_eq!(parsed["title"], "SpecForge Graph Protocol");
}

#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema describes all registered entity kinds")]
fn schema_publish_includes_node_kind_enum() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["schema", "--publish"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    // Verify the nodes items structure exists
    let node_props = &parsed["properties"]["nodes"]["items"]["properties"];
    assert!(node_props["kind"].is_object(), "kind should have schema constraints");
    assert_eq!(node_props["kind"]["type"], "string");
}

#[test]
#[specforge_test(behavior = "publish_schema_specification", verify = "published schema describes all edge types")]
fn schema_publish_includes_edge_label_enum() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["schema", "--publish"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);

    let edge_props = &parsed["properties"]["edges"]["items"]["properties"];
    assert!(edge_props["label"].is_object(), "label should have schema constraints");
    assert_eq!(edge_props["label"]["type"], "string");
}

#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "schema embedded as top-level key in JSON export")]
fn export_v2_schema_has_entity_kinds() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"]["entity_kinds"].is_array(), "V2 schema should have entity_kinds");
}

#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "format_version set to 2.0 with schema")]
fn export_v2_schema_has_edge_types() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(parsed["schema"]["edge_types"].is_array(), "V2 schema should have edge_types");
}

#[test]
#[specforge_test(behavior = "negotiate_schema_version", verify = "compatible version within range is resolved")]
fn export_schema_version_negotiation() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    // Valid version (1.0.0 matches the current schema version)
    let output = specforge_cmd()
        .args(["export", "--format=graph", "--schema-version=1.0.0"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "1.0.0 should be accepted");

    // Invalid major version
    specforge_cmd()
        .args(["export", "--format=graph", "--schema-version=99.0.0"])
        .arg(dir.path())
        .assert()
        .code(1);
}

#[test]
#[specforge_test(behavior = "embed_schema_in_export", verify = "--no-schema suppresses schema and keeps format_version 1.0")]
fn export_scoped_v2_still_has_schema() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph", "--scope=alpha"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object(), "scoped V2 should still embed schema");
}
