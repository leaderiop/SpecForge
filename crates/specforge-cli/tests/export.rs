use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn setup_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }
    dir
}

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

const SPEC_CONTENT: &str = r#"
behavior alpha "Alpha Behavior" {
    contract "The system MUST do alpha"
    status done
}
behavior beta "Beta Behavior" {
    contract "The system MUST do beta"
}
feature gamma "Gamma Feature" {
    behaviors [alpha, beta]
}
"#;

#[test]
fn export_graph_produces_valid_json_with_all_nodes() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "export should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    assert!(parsed["schema_version"].is_string());
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2);
}

#[test]
fn export_brief_produces_minimal_output() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=brief"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    // Brief: no fields, no contract, no file/line
    for node in nodes {
        assert!(node.get("fields").is_none(), "brief should not have fields");
        assert!(node.get("file").is_none(), "brief should not have file");
    }
}

#[test]
fn export_context_includes_contracts() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=context"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    let alpha = nodes.iter().find(|n| n["id"] == "alpha").unwrap();
    assert_eq!(alpha["contract"], "The system MUST do alpha");
}

#[test]
fn export_dot_produces_graphviz() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=dot"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph"));
    assert!(stdout.contains("alpha"));
    assert!(stdout.contains("gamma"));
}

#[test]
fn export_with_errors_still_works() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Export succeeds even with resolution errors (outputs what it can)
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed["nodes"].as_array().unwrap().len() >= 2);
}

#[test]
fn export_with_scope_returns_subgraph() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph", "--scope=alpha"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    // alpha is connected to gamma (via behaviors edge), gamma connects to beta
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"gamma"));
}

#[test]
fn export_with_nonexistent_scope_exits_one() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    specforge_cmd()
        .args(["export", "--format=graph", "--scope=nonexistent"])
        .arg(dir.path())
        .assert()
        .code(1);
}

// B:embed_schema_in_export — V2 format with embedded schema (default)
#[test]
fn export_default_produces_v2_with_schema() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
    assert!(parsed["schema_version"].is_string());
}

// B:embed_schema_in_export — --no-schema suppresses schema and outputs V1 format
#[test]
fn export_no_schema_flag_produces_v1() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=graph", "--no-schema"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(parsed.get("format_version").is_none(), "V1 format has no format_version");
    assert!(parsed.get("schema").is_none(), "V1 format has no schema key");
    assert!(parsed["schema_version"].is_string(), "V1 format has schema_version");
}

// B:embed_schema_in_export — V2 brief with schema
#[test]
fn export_brief_produces_v2_with_schema() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=brief"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:embed_schema_in_export — V2 context with schema
#[test]
fn export_context_produces_v2_with_schema() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["export", "--format=context"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["format_version"], "2.0");
    assert!(parsed["schema"].is_object());
}

// B:negotiate_schema_version — invalid --schema-version exits 1
#[test]
fn export_invalid_schema_version_exits_one() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    specforge_cmd()
        .args(["export", "--format=graph", "--schema-version=invalid"])
        .arg(dir.path())
        .assert()
        .code(1);
}

// B:serve_schema_resource — specforge schema command outputs JSON
#[test]
fn schema_command_outputs_json() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["schema"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(parsed["schema_version"].is_object());
    assert!(parsed["entity_kinds"].is_array());
    assert!(parsed["edge_types"].is_array());
}

// B:publish_schema_specification — specforge schema --publish outputs JSON Schema
#[test]
fn schema_publish_outputs_json_schema() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["schema", "--publish"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert_eq!(parsed["title"], "SpecForge Graph Protocol");
}
