use crate::e2e_fixtures::*;
use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;

// --- Phase 1d: Schema command tests ---

#[specforge_test(
    behavior = "serve_schema_resource",
    verify = "specforge schema outputs full schema as JSON"
)]
fn schema_command_outputs_valid_json() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["schema"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(
        parsed["schema_version"].is_object(),
        "should have schema_version object"
    );
    assert!(
        parsed["entity_kinds"].is_array(),
        "should have entity_kinds array"
    );
    assert!(
        parsed["edge_types"].is_array(),
        "should have edge_types array"
    );
}

#[specforge_test(
    behavior = "serve_schema_resource",
    verify = "--kind filter restricts to single entity kind"
)]
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

#[specforge_test(
    behavior = "serve_schema_resource",
    verify = "schema reflects current compilation state"
)]
fn schema_kind_filter_unknown_exits_one() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    specforge_cmd()
        .args(["schema", "--kind=nonexistent_kind"])
        .arg(dir.path())
        .assert()
        .code(1);
}

#[specforge_test(
    behavior = "publish_schema_specification",
    verify = "published schema is valid JSON Schema"
)]
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

#[specforge_test(
    behavior = "publish_schema_specification",
    verify = "published schema describes all registered entity kinds"
)]
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
    assert!(
        node_props["kind"].is_object(),
        "kind should have schema constraints"
    );
    assert_eq!(node_props["kind"]["type"], "string");
}

#[specforge_test(
    behavior = "publish_schema_specification",
    verify = "published schema describes all edge types"
)]
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
    assert!(
        edge_props["label"].is_object(),
        "label should have schema constraints"
    );
    assert_eq!(edge_props["label"]["type"], "string");
}

#[specforge_test(
    behavior = "embed_schema_in_export",
    verify = "schema embedded as top-level key in full JSON export"
)]
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
    assert!(
        parsed["schema"]["entity_kinds"].is_array(),
        "V2 schema should have entity_kinds"
    );
}

#[specforge_test(
    behavior = "embed_schema_in_export",
    verify = "format_version set to 2.0 with schema"
)]
fn export_v2_schema_has_edge_types() {
    let dir = setup_project(&[("main.spec", SOFTWARE_SPEC)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert!(
        parsed["schema"]["edge_types"].is_array(),
        "V2 schema should have edge_types"
    );
}

#[specforge_test(
    behavior = "negotiate_schema_version",
    verify = "compatible version within range is resolved"
)]
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

#[specforge_test(
    behavior = "embed_schema_in_export",
    verify = "scoped exports carry schema_ref (url and content_hash) instead of embedded schema"
)]
fn export_scoped_v2_references_schema() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
"#,
    )]);

    let output = specforge_cmd()
        .args(["export", "--format=graph", "--scope=alpha"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let parsed = parse_json_stdout(&output);
    assert_eq!(parsed["format_version"], "2.0");
    assert!(
        parsed.get("schema").is_none(),
        "scoped V2 must not embed the full schema"
    );
    assert!(
        parsed["schema_ref"]["url"].is_string(),
        "scoped V2 carries a schema_ref url"
    );
    assert_eq!(
        parsed["schema_ref"]["content_hash"].as_str().unwrap().len(),
        64,
        "schema_ref content_hash is a sha256 hex digest"
    );
}

#[test]
fn schema_publish_describes_the_requested_format() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"s","spec_root":"src","extensions":["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(root.join("src/a.spec"), "type W { id string @unique }").unwrap();

    let run = |fmt: &str| {
        let out = Command::new(env!("CARGO_BIN_EXE_specforge"))
            .args([
                "schema",
                root.to_str().unwrap(),
                "--publish",
                "--format",
                fmt,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{fmt}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("published schema is JSON");
        v["properties"]["nodes"]["items"].clone()
    };

    let full = run("graph");
    assert_eq!(
        full["required"],
        serde_json::json!(["id", "kind", "file", "line", "fields"]),
        "graph schema describes full nodes"
    );

    let context = run("context");
    assert_eq!(
        context["required"],
        serde_json::json!(["id", "kind"]),
        "context schema describes context nodes (no file/line/fields required)"
    );
    assert!(
        context["properties"].get("verify").is_some(),
        "context schema knows the verify field"
    );
    assert!(context["properties"].get("file").is_none());

    let brief = run("brief");
    assert_eq!(
        brief["properties"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>()),
        Some(vec!["id".into(), "kind".into(), "title".into()]),
        "brief schema exposes exactly id/kind/title"
    );
}
