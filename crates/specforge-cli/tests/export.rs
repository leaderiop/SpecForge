use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
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
    let dir = setup_project(&[
        (
            "specforge.json",
            r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
        ),
        ("main.spec", SPEC_CONTENT),
    ]);

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
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

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

#[specforge_test(
    behavior = "embed_schema_in_export",
    verify = "--no-schema suppresses schema and keeps format_version 1.0"
)]
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

    assert_eq!(parsed["format_version"], "1.0", "{parsed}");
    assert!(
        parsed.get("schema").is_none(),
        "V1 format has no schema key"
    );
    assert!(
        parsed["schema_version"].is_string(),
        "V1 format has schema_version"
    );
}

#[specforge_test(
    behavior = "export_agent_brief_format",
    verify = "the brief export leaves the schema out unless --with-schema is given"
)]
fn export_brief_omits_the_schema_unless_asked() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);
    let export = |extra: &[&str]| {
        let output = specforge_cmd()
            .args(["export", "--format=brief"])
            .args(extra)
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };

    let lean = export(&[]);
    assert!(lean.get("schema").is_none(), "{lean}");
    assert!(!lean["nodes"].as_array().unwrap().is_empty());

    let full = export(&["--with-schema"]);
    assert_eq!(full["format_version"], "2.0");
    assert!(full["schema"].is_object());
}

#[specforge_test(
    behavior = "export_agent_context_format",
    verify = "the context export leaves the schema out unless --with-schema is given"
)]
fn export_context_omits_the_schema_unless_asked() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);
    let export = |extra: &[&str]| {
        let output = specforge_cmd()
            .args(["export", "--format=context"])
            .args(extra)
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };

    let lean = export(&[]);
    assert!(lean.get("schema").is_none(), "{lean}");
    assert!(!lean["nodes"].as_array().unwrap().is_empty());

    let full = export(&["--with-schema"]);
    assert_eq!(full["format_version"], "2.0");
    assert!(full["schema"].is_object());
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

    assert_eq!(
        parsed["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(parsed["title"], "SpecForge Graph Protocol");
}

#[specforge_test(
    behavior = "export_agent_context_format",
    verify = "export --format context keeps an invariant's guarantee"
)]
fn export_context_keeps_an_invariants_guarantee() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("app.spec"),
        "invariant unique_ids \"Unique ids\" {\n  guarantee \"Ids MUST be unique\"\n  risk \"duplicates\"\n}\n",
    )
    .unwrap();
    let out = specforge_cmd()
        .args(["export", "--format", "context"])
        .arg(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let node = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "unique_ids")
        .unwrap_or_else(|| panic!("no unique_ids in {parsed}"));
    assert_eq!(node["fields"]["guarantee"], "Ids MUST be unique", "{node}");
    assert!(
        node["fields"].get("risk").is_none(),
        "risk isn't normative: {node}"
    );
}

// ── --max-tokens on the graph export ──────────────────────────────────

/// Nine entities, each well over 100 tokens: eight behaviors with long
/// contracts and a feature linking them all. Two extensions give the export
/// a schema of real size.
fn budget_project() -> TempDir {
    const CONFIG: &str = r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/product"]}"#;
    let mut spec = String::new();
    for i in 0..8 {
        spec.push_str(&format!(
            "behavior b{i} \"Behavior {i}\" {{\n  contract \"The system MUST {}\"\n}}\n",
            "handle this case with care ".repeat(20)
        ));
    }
    spec.push_str(
        "feature f \"All behaviors\" {\n  behaviors [b0, b1, b2, b3, b4, b5, b6, b7]\n}\n",
    );
    setup_project(&[("specforge.json", CONFIG), ("main.spec", &spec)])
}

/// `specforge export <dir> --format=graph <args>`: the exit code, stdout and
/// stderr.
fn export_graph(dir: &TempDir, args: &[&str]) -> (i32, String, String) {
    let output = specforge_cmd()
        .args(["export", "--format=graph"])
        .args(args)
        .arg(dir.path())
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// The embedded schema of this project's graph export, and what it costs by
/// the estimator the budget uses.
fn schema_tokens(dir: &TempDir) -> (serde_json::Value, usize) {
    let (code, stdout, stderr) = export_graph(dir, &["--with-schema"]);
    assert_eq!(code, 0, "{stderr}");
    let full: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let schema = full["schema"].clone();
    assert!(schema.is_object(), "{full}");
    let tokens = specforge_emitter::estimate_tokens(&serde_json::to_string(&schema).unwrap());
    (schema, tokens)
}

#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "the graph export honours --max-tokens with the schema left out unless --with-schema is given"
)]
fn graph_export_honours_max_tokens_without_the_schema() {
    let dir = budget_project();
    let budget = 500;

    let (code, stdout, stderr) = export_graph(&dir, &["--max-tokens", &budget.to_string()]);
    assert_eq!(code, 0, "{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let used = specforge_emitter::estimate_tokens(&stdout);
    assert!(used <= budget, "{used} tokens over the {budget} budget");
    assert!(parsed.get("schema").is_none(), "{parsed}");
    assert_eq!(parsed["format_version"], "1.0", "{parsed}");
    let kept = parsed["nodes"].as_array().unwrap().len();
    assert!(kept > 0 && kept < 9, "kept {kept} of 9: {parsed}");
    let truncated = parsed["token_budget"]["truncated_entities"]
        .as_array()
        .unwrap();
    assert_eq!(kept + truncated.len(), 9, "{parsed}");
}

#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "an embedded schema counts toward the token budget"
)]
fn embedded_schema_counts_toward_the_budget() {
    let dir = budget_project();
    let (schema, schema_cost) = schema_tokens(&dir);
    // The schema plus room for a few entities, not all nine.
    let budget = schema_cost + 400;

    let (code, stdout, stderr) = export_graph(
        &dir,
        &["--with-schema", "--max-tokens", &budget.to_string()],
    );
    assert_eq!(code, 0, "{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let used = specforge_emitter::estimate_tokens(&stdout);
    assert!(used <= budget, "{used} tokens over the {budget} budget");
    assert_eq!(parsed["format_version"], "2.0", "{parsed}");
    assert_eq!(parsed["schema"], schema, "the schema travels whole");
    let kept = parsed["nodes"].as_array().unwrap().len();
    assert!(kept > 0 && kept < 9, "kept {kept} of 9");
    assert_eq!(
        parsed["token_budget"]["truncated_entities"]
            .as_array()
            .unwrap()
            .len(),
        9 - kept
    );
}

#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "a budget smaller than the embedded schema fails with E062 instead of truncating the schema"
)]
fn budget_smaller_than_the_schema_fails() {
    let dir = budget_project();
    let (_, schema_cost) = schema_tokens(&dir);
    let budget = schema_cost - 1;

    let (code, stdout, stderr) = export_graph(
        &dir,
        &["--with-schema", "--max-tokens", &budget.to_string()],
    );
    assert_eq!(code, 1, "stdout: {stdout}");
    assert!(stdout.trim().is_empty(), "no partial export: {stdout}");
    assert!(stderr.contains("E062"), "{stderr}");
    assert!(stderr.contains("schema"), "{stderr}");

    // Without --with-schema the same budget is plenty.
    let (code, _, stderr) = export_graph(&dir, &["--max-tokens", &budget.to_string()]);
    assert_eq!(code, 0, "{stderr}");
}

#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "a budget below one entity yields the envelope with no entities and the truncation marker"
)]
fn budget_below_one_entity_yields_an_empty_envelope() {
    let dir = budget_project();
    // Every entity costs more than 100 tokens; the empty envelope less.
    let budget = 100;

    let (code, stdout, stderr) = export_graph(&dir, &["--max-tokens", &budget.to_string()]);
    assert_eq!(code, 0, "{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let used = specforge_emitter::estimate_tokens(&stdout);
    assert!(used <= budget, "{used} tokens over the {budget} budget");
    assert_eq!(parsed["format_version"], "1.0", "{parsed}");
    assert!(parsed["schema_version"].is_string(), "{parsed}");
    assert_eq!(parsed["nodes"], serde_json::json!([]), "{parsed}");
    assert_eq!(parsed["edges"], serde_json::json!([]), "{parsed}");
    let meta = &parsed["token_budget"];
    assert_eq!(meta["strategy"], "prioritize", "{parsed}");
    assert_eq!(meta["budget_tokens"], budget, "{parsed}");
    let mut truncated: Vec<&str> = meta["truncated_entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    truncated.sort();
    assert_eq!(
        truncated,
        vec!["b0", "b1", "b2", "b3", "b4", "b5", "b6", "b7", "f"]
    );
}

#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "a budget below the empty envelope fails with E062"
)]
fn budget_below_the_empty_envelope_fails() {
    let dir = budget_project();

    let (code, stdout, stderr) = export_graph(&dir, &["--max-tokens", "5"]);
    assert_eq!(code, 1, "stdout: {stdout}");
    assert!(stdout.trim().is_empty(), "no over-budget export: {stdout}");
    assert!(stderr.contains("E062"), "{stderr}");
}
