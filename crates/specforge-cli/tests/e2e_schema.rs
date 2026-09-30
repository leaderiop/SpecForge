use crate::e2e_fixtures::*;
use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::collections::BTreeSet;

/// SOFTWARE_SPEC in a project that enables @specforge/software, so the
/// schema has registered kinds and edge types.
fn software_project() -> tempfile::TempDir {
    setup_project_with_config(
        r#"{"name":"t","version":"0.1.0","spec_root":"spec","extensions":["@specforge/software"]}"#,
        &[("main.spec", SOFTWARE_SPEC)],
    )
}

/// The kinds @specforge/software registers.
const SOFTWARE_KINDS: [&str; 5] = ["behavior", "event", "invariant", "port", "type"];

fn run_json(args: &[&str], dir: &std::path::Path) -> serde_json::Value {
    let output = specforge_cmd().args(args).arg(dir).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_json_stdout(&output)
}

fn strings(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn names(list: &serde_json::Value, key: &str) -> BTreeSet<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|v| v[key].as_str().unwrap().to_string())
        .collect()
}

// --- Phase 1d: Schema command tests ---

#[specforge_test(
    behavior = "serve_schema_resource",
    verify = "specforge schema outputs full schema as JSON"
)]
fn schema_command_outputs_valid_json() {
    let dir = software_project();
    let parsed = run_json(&["schema"], dir.path());

    assert_eq!(
        parsed["schema_version"],
        serde_json::json!({"major": 1, "minor": 0, "patch": 0})
    );
    assert_eq!(
        names(&parsed["extensions"], "name"),
        strings(&serde_json::json!(["@specforge/software"]))
    );
    // Every kind the extension registers, with its field definitions.
    assert_eq!(
        names(&parsed["entity_kinds"], "name"),
        SOFTWARE_KINDS.iter().map(|s| s.to_string()).collect()
    );
    let behavior = parsed["entity_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["name"] == "behavior")
        .unwrap();
    let contract = behavior["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "contract")
        .expect("behavior.contract");
    assert_eq!(contract["field_type"], "string");
    assert_eq!(contract["source_extension"], "@specforge/software");
    // And its edge types.
    let edge_types = names(&parsed["edge_types"], "label");
    for label in [
        "BehaviorEnforcesInvariant",
        "BehaviorProducesEvent",
        "BehaviorUsesPort",
    ] {
        assert!(
            edge_types.contains(label),
            "{label} missing: {edge_types:?}"
        );
    }
}

#[test]
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
    for dir in [
        software_project(),
        setup_project(&[("main.spec", SOFTWARE_SPEC)]),
    ] {
        let parsed = run_json(&["schema", "--publish"], dir.path());
        assert_eq!(
            parsed["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "published schema should have $schema field"
        );
        assert_eq!(parsed["title"], "SpecForge Graph Protocol");
        // Every keyword is a draft 2020-12 keyword with a well-typed value.
        let defs = parsed["$defs"]
            .as_object()
            .map(|d| d.keys().cloned().collect())
            .unwrap_or_default();
        assert_json_schema(&parsed, "#", &defs);
    }
}

/// Assert `node` is a well-formed JSON Schema (draft 2020-12) for the
/// keywords the published schema uses; `defs` are the `$defs` names.
fn assert_json_schema(node: &serde_json::Value, path: &str, defs: &BTreeSet<String>) {
    use serde_json::Value;
    const TYPES: [&str; 7] = [
        "object", "array", "string", "integer", "number", "boolean", "null",
    ];
    let obj = match node {
        Value::Bool(_) => return,
        Value::Object(obj) => obj,
        other => panic!("{path}: a schema must be an object or boolean, got {other}"),
    };
    for (key, value) in obj {
        let at = format!("{path}/{key}");
        match key.as_str() {
            "$schema" | "$id" | "title" | "description" | "$comment" | "pattern" | "format" => {
                assert!(value.is_string(), "{at} must be a string")
            }
            "type" => {
                let listed: Vec<&Value> = match value {
                    Value::Array(items) => items.iter().collect(),
                    single => vec![single],
                };
                for t in listed {
                    assert!(
                        t.as_str().is_some_and(|t| TYPES.contains(&t)),
                        "{at}: unknown type {t}"
                    );
                }
            }
            "properties" | "$defs" => {
                for (name, sub) in value.as_object().unwrap_or_else(|| panic!("{at}: object")) {
                    assert_json_schema(sub, &format!("{at}/{name}"), defs);
                }
            }
            "items"
            | "additionalProperties"
            | "if"
            | "then"
            | "else"
            | "not"
            | "contains"
            | "propertyNames" => assert_json_schema(value, &at, defs),
            "allOf" | "anyOf" | "oneOf" => {
                let items = value.as_array().unwrap_or_else(|| panic!("{at}: array"));
                assert!(!items.is_empty(), "{at}: empty");
                for (i, sub) in items.iter().enumerate() {
                    assert_json_schema(sub, &format!("{at}/{i}"), defs);
                }
            }
            "minItems" | "maxItems" | "minLength" | "maxLength" | "minProperties" => {
                assert!(value.is_u64(), "{at} must be a non-negative integer")
            }
            "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" => {
                assert!(value.is_number(), "{at} must be a number")
            }
            "uniqueItems" => assert!(value.is_boolean(), "{at} must be a boolean"),
            "required" => {
                // Inside a sub-schema (then/allOf) `required` may name
                // properties declared by the enclosing schema.
                let props = obj.get("properties").and_then(Value::as_object);
                for name in value.as_array().unwrap_or_else(|| panic!("{at}: array")) {
                    let name = name.as_str().unwrap_or_else(|| panic!("{at}: strings"));
                    if let Some(props) = props {
                        assert!(
                            props.contains_key(name),
                            "{at}: required '{name}' is not a declared property"
                        );
                    }
                }
            }
            "enum" => {
                let items = value.as_array().unwrap_or_else(|| panic!("{at}: array"));
                assert!(!items.is_empty(), "{at}: empty enum");
                let unique: BTreeSet<String> = items.iter().map(|v| v.to_string()).collect();
                assert_eq!(unique.len(), items.len(), "{at}: duplicate enum values");
            }
            "const" => {}
            "$ref" => {
                let target = value.as_str().unwrap_or_default();
                let name = target
                    .strip_prefix("#/$defs/")
                    .unwrap_or_else(|| panic!("{at}: unsupported $ref {target}"));
                assert!(defs.contains(name), "{at}: $ref to missing {target}");
            }
            other => panic!("{at}: unexpected keyword '{other}'"),
        }
    }
}

#[specforge_test(
    behavior = "publish_schema_specification",
    verify = "published schema describes all registered entity kinds"
)]
fn schema_publish_includes_node_kind_enum() {
    let dir = software_project();
    let parsed = run_json(&["schema", "--publish"], dir.path());

    let kind = &parsed["properties"]["nodes"]["items"]["properties"]["kind"];
    assert_eq!(kind["type"], "string");
    assert_eq!(
        strings(&kind["enum"]),
        SOFTWARE_KINDS.iter().map(|s| s.to_string()).collect(),
        "the node kind enum lists every registered kind"
    );
    // The same set `specforge schema` reports as registered.
    let registered = run_json(&["schema"], dir.path());
    assert_eq!(
        strings(&kind["enum"]),
        names(&registered["entity_kinds"], "name")
    );
}

#[specforge_test(
    behavior = "publish_schema_specification",
    verify = "published schema describes all edge types"
)]
fn schema_publish_includes_edge_label_enum() {
    let dir = software_project();
    let parsed = run_json(&["schema", "--publish"], dir.path());

    let label = &parsed["properties"]["edges"]["items"]["properties"]["label"];
    assert_eq!(label["type"], "string");
    let listed = strings(&label["enum"]);
    // Every edge type `specforge schema` reports as registered, no more.
    let registered = run_json(&["schema"], dir.path());
    assert_eq!(listed, names(&registered["edge_types"], "label"));
    for expected in [
        "BehaviorEnforcesInvariant",
        "BehaviorProducesEvent",
        "BehaviorConsumesEvent",
        "BehaviorUsesPort",
        "EventCarriesPayloadType",
    ] {
        assert!(listed.contains(expected), "{expected} missing: {listed:?}");
    }
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
    let dir = software_project();
    let parsed = run_json(&["export", "--format=graph"], dir.path());

    // Format 2.0, carrying the project's schema.
    assert_eq!(parsed["format_version"], "2.0");
    let registered = run_json(&["schema"], dir.path());
    assert_eq!(
        parsed["schema"], registered,
        "the embedded schema is the project's"
    );
    assert!(
        names(&parsed["schema"]["edge_types"], "label").contains("BehaviorEnforcesInvariant"),
        "{}",
        parsed["schema"]["edge_types"]
    );

    // Without the schema, the export is not 2.0.
    let bare = run_json(&["export", "--format=graph", "--no-schema"], dir.path());
    assert!(bare.get("schema").is_none());
    assert_ne!(bare["format_version"], "2.0");
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
