//! What the two surfaces serve from a project's extension commands:
//! `@specforge/product`'s MCP tools as `tools/list` lists them, and its
//! command lines as `specforge product <command> --help` shows them.

use crate::e2e_fixtures::{find_response, mcp_request};
use crate::product_commands::{setup_product_project, structured_session};
use assert_cmd::cargo_bin_cmd;
use serde_json::Value;
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;

const PRODUCT: &str = "@specforge/product";

/// Every tool `specforge mcp` lists for the product project, in order.
fn listed_tools(dir: &TempDir) -> Vec<Value> {
    let responses = structured_session(dir, &[mcp_request(1, "tools/list", serde_json::json!({}))]);
    find_response(&responses, 1).unwrap()["result"]["tools"]
        .as_array()
        .unwrap()
        .clone()
}

/// The product command a product tool runs, as the command line names it.
fn cli_name(tool: &Value) -> String {
    let name = tool["name"].as_str().unwrap();
    let id = name
        .strip_prefix("specforge.product.")
        .unwrap_or_else(|| panic!("{name} is not a product tool"));
    id.replace('_', "-")
}

/// `specforge product <args>`'s stdout, at a fixed width and without
/// colour.
fn product_help(dir: &TempDir, args: &[&str]) -> String {
    let output = cargo_bin_cmd!("specforge")
        .env("COLUMNS", "100")
        .env("NO_COLOR", "1")
        .arg("product")
        .args(args)
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn pinned_product_tools_listing() {
    let dir = setup_product_project();
    let tools = listed_tools(&dir);
    // The listed order, core first: pinned apart from the schemas.
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    insta::assert_json_snapshot!("product_tool_names", names);
    let product: Vec<&Value> = tools.iter().filter(|t| t["source"] == PRODUCT).collect();
    insta::assert_json_snapshot!("product_tools", product);
}

#[test]
fn pinned_product_command_help() {
    let dir = setup_product_project();
    // `specforge product <unknown>` would print the command list too, but
    // as an error: `--help` lists them.
    insta::assert_snapshot!("help_product", product_help(&dir, &["--help"]));
    let tools = listed_tools(&dir);
    let commands: Vec<String> = tools
        .iter()
        .filter(|t| t["source"] == PRODUCT)
        .map(cli_name)
        .collect();
    assert!(!commands.is_empty());
    for command in commands {
        let help = product_help(&dir, &[&command, "--help"]);
        insta::assert_snapshot!(format!("help_{command}"), help);
    }
}

/// `specforge product <args> --format json`'s stdout, as JSON.
fn product_json(dir: &TempDir, args: &[&str]) -> Value {
    let output = cargo_bin_cmd!("specforge")
        .arg("product")
        .args(args)
        .args(["--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "the CLI and MCP send a command's export the same args for the same input, its declared defaults applied by the host"
)]
fn the_cli_and_mcp_answer_one_command_alike() {
    let dir = setup_product_project();
    let calls = [
        (vec!["features"], serde_json::json!({})),
        (
            vec!["features", "--status", "done", "--limit", "1"],
            serde_json::json!({"status": "done", "limit": 1}),
        ),
        (
            vec!["milestone-completion", "m1"],
            serde_json::json!({"milestone": "m1"}),
        ),
    ];
    let requests: Vec<String> = calls
        .iter()
        .enumerate()
        .map(|(i, (argv, arguments))| {
            let name = format!("specforge.product.{}", argv[0].replace('-', "_"));
            mcp_request(
                i as u64 + 1,
                "tools/call",
                serde_json::json!({"name": name, "arguments": arguments}),
            )
        })
        .collect();
    let responses = structured_session(&dir, &requests);
    for (i, (argv, _)) in calls.iter().enumerate() {
        let response = find_response(&responses, i as u64 + 1).unwrap();
        assert_eq!(response["result"]["isError"], false, "{argv:?}: {response}");
        assert_eq!(
            response["result"]["structuredContent"],
            product_json(&dir, argv),
            "{argv:?}"
        );
    }
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP an argument the command's declaration refuses is the INVALID_INPUT error object the CLI writes, and the export is not called"
)]
fn one_mistake_is_one_error_object_on_both_surfaces() {
    let dir = setup_product_project();
    let mistakes = [
        (
            vec!["features", "--limit", "-1"],
            serde_json::json!({"limit": -1}),
        ),
        (vec!["milestone-completion"], serde_json::json!({})),
        (
            vec!["features", "--status", "bogus"],
            serde_json::json!({"status": "bogus"}),
        ),
    ];
    let requests: Vec<String> = mistakes
        .iter()
        .enumerate()
        .map(|(i, (argv, arguments))| {
            let name = format!("specforge.product.{}", argv[0].replace('-', "_"));
            mcp_request(
                i as u64 + 1,
                "tools/call",
                serde_json::json!({"name": name, "arguments": arguments}),
            )
        })
        .collect();
    let responses = structured_session(&dir, &requests);
    for (i, (argv, _)) in mistakes.iter().enumerate() {
        let output = cargo_bin_cmd!("specforge")
            .arg("product")
            .args(argv)
            .args(["--format", "json", "--path"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{argv:?}: {output:?}");
        let cli: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(cli["code"], "INVALID_INPUT", "{argv:?}: {cli}");
        let response = find_response(&responses, i as u64 + 1).unwrap();
        assert_eq!(response["result"]["isError"], true, "{argv:?}: {response}");
        assert_eq!(response["result"]["structuredContent"], cli, "{argv:?}");
    }
}
