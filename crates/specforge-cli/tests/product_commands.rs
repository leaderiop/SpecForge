//! The `specforge product <command>` commands: `@specforge/product`'s
//! `cmd__product_*` exports, which the CLI routes to from the commands the
//! extension declares (ADR 0008). Output, flags and names are the ones the
//! built-in `product` subcommands had.

use crate::e2e_fixtures::{
    find_response, mcp_raw_session_in, mcp_request, mcp_session_in, parse_tool_content,
};
use assert_cmd::cargo_bin_cmd;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn setup_product_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": ["@specforge/product"]
    });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();

    fs::write(
        dir.path().join("spec.spec"),
        r#"persona dev "Developer" {
    description "A software developer"
    status active
}

channel cli "CLI" {
    description "Command-line interface"
    status active
}

feature f1 "Core Feature" {
    status proposed
    priority high
}

feature f2 "Secondary Feature" {
    status done
    priority medium
    depends_on [f1]
}

journey j1 "Dev Flow" {
    persona dev
    channels [cli]
    features [f1, f2]
    description "Developer uses CLI"
}

module mod1 "Core Module" {
    features [f1]
}

milestone m1 "Launch" {
    status planned
    features [f1, f2]
    modules [mod1]
}

deliverable d1 "CLI App" {
    artifact_type cli
    status draft
    journeys [j1]
    modules [mod1]
    milestones [m1]
}

term specification "Specification" {
    definition "A formal description of system behavior"
}

release r1 "Initial Release" {
    version "1.0.0"
    status planned
    deliverables [d1]
    milestones [m1]
}
"#,
    )
    .unwrap();

    dir
}

#[test]
fn test_product_features_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "features",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 2);
}

#[test]
fn test_product_features_filter_status() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "features",
        "--path",
        dir.path().to_str().unwrap(),
        "--status",
        "proposed",
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
    assert_eq!(result["entities"][0]["id"], "f1");
}

#[test]
fn test_product_milestones_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "milestones",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_milestone_completion() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "milestone-completion",
        "m1",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["milestone_id"], "m1");
    assert_eq!(result["total_features"], 2);
    assert_eq!(result["done_features"], 1); // f2 is done
}

#[test]
fn test_product_feature_impact() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "feature-impact",
        "f1",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["feature_id"], "f1");
    assert!(
        !result["referenced_by_journeys"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !result["referenced_by_milestones"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !result["referenced_by_modules"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[specforge_test(
    behavior = "pe_query_feature_impact",
    verify = "a feature that only relates to the feature is not a dependent"
)]
fn a_related_feature_is_not_a_dependent_in_the_impact() {
    let dir = setup_product_project();
    // f3 relates to f1 (`features`); f2 depends on it (`depends_on`).
    fs::write(
        dir.path().join("related.spec"),
        "feature f3 \"Related Feature\" {\n    status proposed\n    features [f1]\n}\n",
    )
    .unwrap();
    let output = cargo_bin_cmd!("specforge")
        .args([
            "product",
            "feature-impact",
            "f1",
            "--format",
            "json",
            "--path",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result["depended_on_by"],
        serde_json::json!(["f2"]),
        "{result}"
    );
}

#[test]
fn test_product_feature_dependents() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "feature-dependents",
        "f1",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let deps = result.as_array().unwrap();
    assert!(
        deps.iter().any(|v| v == "f2"),
        "f2 depends on f1: {:?}",
        deps
    );
}

#[test]
fn test_product_health() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "health",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(result["score"]["overall"].as_f64().unwrap() > 0.0);
    assert!(!result["entity_counts"].as_array().unwrap().is_empty());
}

#[test]
fn test_product_modules_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "modules",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_terms_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "terms",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_releases_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "releases",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_personas_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "personas",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_channels_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "channels",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_journey_coverage() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "journey-coverage",
        "j1",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["journey_id"], "j1");
    assert_eq!(result["total_features"], 2);
    assert_eq!(result["covered_by_modules"], 1); // f1 is in mod1
}

#[test]
fn test_product_persona_features() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "persona-features",
        "dev",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let features = result.as_array().unwrap();
    assert_eq!(features.len(), 2); // f1 and f2 via journey j1
}

#[test]
fn test_product_channel_features() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "channel-features",
        "cli",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let features = result.as_array().unwrap();
    assert_eq!(features.len(), 2); // f1 and f2 via journey j1
}

#[test]
fn test_product_bulk_status() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "bulk-status",
        "--path",
        dir.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = result.as_array().unwrap();
    // Should have entries for feature, milestone, deliverable, persona, channel, release
    assert!(
        arr.len() >= 4,
        "Expected at least 4 status-bearing kinds, got {}",
        arr.len()
    );
}

#[test]
fn test_product_nonexistent_milestone_exits_one() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args([
        "product",
        "milestone-completion",
        "nonexistent",
        "--path",
        dir.path().to_str().unwrap(),
    ]);
    cmd.assert().failure();
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "exit code, stdout, stderr returned to CLI"
)]
fn an_extension_command_prints_what_its_export_returns() {
    let dir = setup_product_project();
    let path = dir.path().to_str().unwrap();
    let output = cargo_bin_cmd!("specforge")
        .args(["product", "milestone-completion", "m1", "--path", path])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Milestone: m1 (planned)\nCompletion: 50% (1/2 features done)\n  f1 [proposed]\n  f2 [done]\n"
    );
    assert!(output.stderr.is_empty(), "{output:?}");

    let output = cargo_bin_cmd!("specforge")
        .args(["product", "milestone-completion", "nope", "--path", path])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "milestone 'nope' not found\n"
    );
}

#[test]
fn an_extension_command_has_the_declared_command_line() {
    let dir = setup_product_project();
    let path = dir.path().to_str().unwrap();
    // `ext:command` names the same command as `ext command`.
    let output = cargo_bin_cmd!("specforge")
        .args(["product:features", "--path", path, "--limit", "1"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "2 feature entities (showing 1):\n  f1 Core Feature [proposed] pri=high in=4 out=0\n"
    );
    // The declared enum refuses other values; an undeclared flag is refused.
    for args in [
        ["product", "features", "--format", "xml"],
        ["product", "journeys", "--status", "done"],
    ] {
        let output = cargo_bin_cmd!("specforge")
            .args(args)
            .args(["--path", path])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
    }
    // A name no built-in command or extension has is refused.
    let output = cargo_bin_cmd!("specforge")
        .args(["nonesuch", "features", "--path", path])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand 'nonesuch'"),
        "{output:?}"
    );
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "status filter reduces result set"
)]
fn the_features_command_is_an_mcp_tool_with_its_filters() {
    let dir = setup_product_project();
    let call = |id: u64, args: serde_json::Value| {
        mcp_request(
            id,
            "tools/call",
            serde_json::json!({"name": "specforge.product.features", "arguments": args}),
        )
    };
    let responses = mcp_session_in(
        &dir,
        &[
            call(1, serde_json::json!({})),
            call(2, serde_json::json!({"status": "done"})),
        ],
    );
    let all = parse_tool_content(find_response(&responses, 1).unwrap());
    assert_eq!(all["total"], 2, "{all}");
    let done = parse_tool_content(find_response(&responses, 2).unwrap());
    assert_eq!(done["total"], 1, "{done}");
    assert_eq!(done["entities"][0]["id"], "f2");
}

#[specforge_test(
    behavior = "surface_list_features",
    verify = "pagination offset and limit are respected"
)]
fn the_features_command_pages_after_counting() {
    let dir = setup_product_project();
    let output = cargo_bin_cmd!("specforge")
        .args(["product", "features", "--offset", "1", "--limit", "1"])
        .args(["--format", "json", "--path", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["total"], 2);
    let ids: Vec<&str> = result["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["f2"]);
}

#[test]
fn completions_include_the_commands_of_the_project_here() {
    let dir = setup_product_project();
    let output = cargo_bin_cmd!("specforge")
        .current_dir(dir.path())
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let script = String::from_utf8_lossy(&output.stdout);
    assert!(
        script.contains("specforge__subcmd__product__subcmd__milestone__subcmd__completion"),
        "the product commands are completed"
    );

    // Outside a project, the built-ins only.
    let empty = TempDir::new().unwrap();
    let output = cargo_bin_cmd!("specforge")
        .current_dir(empty.path())
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let script = String::from_utf8_lossy(&output.stdout);
    assert!(script.contains("specforge__subcmd__check"));
    assert!(!script.contains("specforge__subcmd__product"));
}

/// `requests` after an `initialize` negotiating MCP 2025-06-18, the first
/// revision with structured content.
fn structured_session(dir: &TempDir, requests: &[String]) -> Vec<serde_json::Value> {
    let initialize = mcp_request(
        0,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "e2e", "version": "0"}
        }),
    );
    let mut all = vec![initialize];
    all.extend_from_slice(requests);
    mcp_raw_session_in(dir, &all)
}

fn tool_call(id: u64, name: &str, args: serde_json::Value) -> String {
    mcp_request(
        id,
        "tools/call",
        serde_json::json!({"name": name, "arguments": args}),
    )
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP a command is asked for json and its JSON output is the tool's structured content"
)]
fn over_mcp_a_command_answers_json_as_structured_content() {
    let dir = setup_product_project();
    let responses = structured_session(
        &dir,
        &[
            mcp_request(1, "tools/list", serde_json::json!({})),
            tool_call(
                2,
                "specforge.product.milestone_completion",
                serde_json::json!({"milestone": "m1"}),
            ),
        ],
    );
    // The tool has no format argument: the host always asks for json.
    let tools = &find_response(&responses, 1).unwrap()["result"]["tools"];
    let tool = tools
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "specforge.product.milestone_completion")
        .unwrap();
    assert!(
        tool["inputSchema"]["properties"].get("format").is_none(),
        "{tool}"
    );

    let response = find_response(&responses, 2).unwrap();
    let result = &response["result"];
    assert_eq!(result["isError"], false, "{response}");
    let structured = &result["structuredContent"];
    assert_eq!(structured["milestone_id"], "m1", "{response}");
    assert_eq!(parse_tool_content(response), *structured);
}

#[specforge_test(
    behavior = "surface_format_conventions",
    verify = "an MCP tool call returns the json payload"
)]
fn an_mcp_tool_call_returns_the_json_payload() {
    let dir = setup_product_project();
    let output = cargo_bin_cmd!("specforge")
        .args(["product", "health", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    let cli: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // Asked for nothing, the tool answers what --format json prints.
    let responses = mcp_session_in(
        &dir,
        &[tool_call(
            1,
            "specforge.product.health",
            serde_json::json!({}),
        )],
    );
    assert_eq!(
        parse_tool_content(find_response(&responses, 1).unwrap()),
        cli
    );
}
