//! A held path keeps the caller's spelling (ADR 0272): a refusal over MCP names a path as the call wrote it,
//! the text the CLI prints for the same argument. Each pin is `#[ignore]`d until the ticket named in its
//! reason flips it.

use crate::support::*;
use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test_macros::test as specforge_test;

fn served() -> Served {
    TestProject::new()
        .file("test.spec", "behavior alpha \"Alpha\" {\n}\n")
        .serve(&[TestExtension::software()])
}

fn refused(server: &mut McpServer, tool: &str, arguments: Value) -> Value {
    crate::tool_errors::mcp_error(&call_tool(server, tool, arguments))
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a refusal of test_results names it as the call spelled it"
)]
#[ignore = "flipped by T3"]
fn analyze_names_test_results_as_the_call_wrote_it() {
    let mut server = served();
    let error = refused(
        &mut server,
        "specforge.analyze",
        json!({"test_results": "none.json"}),
    );
    assert_eq!(error["argument"], "test_results", "{error}");
    assert_eq!(
        error["message"],
        "cannot read test results none.json: No such file or directory (os error 2)",
        "{error}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "a refusal of a local path in specifiers names it as the call spelled it"
)]
#[ignore = "flipped by T4"]
fn add_extension_names_a_local_path_as_the_call_wrote_it() {
    let mut server = served();
    let missing = refused(
        &mut server,
        "specforge.add_extension",
        json!({"specifiers": ["nope.wasm"], "dry_run": true}),
    );
    assert_eq!(
        missing["message"], "no extension binary at nope.wasm",
        "{missing}"
    );
    let outside = refused(
        &mut server,
        "specforge.add_extension",
        json!({"specifiers": ["../outside.wasm"], "dry_run": true}),
    );
    let message = outside["message"].as_str().unwrap();
    assert!(
        message.starts_with("'../outside.wasm' resolves to"),
        "{message}"
    );
}

#[specforge_test(
    invariant = "mcp_structured_error_responses",
    verify = "a call target's refusal names its path as the call spelled it"
)]
#[ignore = "flipped by T5"]
fn a_call_targets_path_is_named_as_the_call_wrote_it() {
    let mut server = served();
    for (tool, arguments) in [
        ("specforge.validate", json!({"path": "nope"})),
        ("specforge.collect", json!({"path": "nope"})),
        (
            "specforge.migrate",
            json!({"path": "nope", "dry_run": true}),
        ),
    ] {
        let error = refused(&mut server, tool, arguments);
        assert_eq!(error["message"], "path not found: nope", "{tool}: {error}");
    }
    let inside = refused(&mut server, "specforge.init", json!({"path": "spec/x"}));
    let message = inside["message"].as_str().unwrap();
    assert!(
        message.starts_with("spec/x is inside the current project at"),
        "{message}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a test_results that names a directory or nothing is invalid_input on test_results"
)]
#[ignore = "flipped by T7"]
fn analyze_refuses_a_test_results_directory_or_empty_name_on_test_results() {
    let mut server = served();
    for named in ["", "."] {
        let error = refused(
            &mut server,
            "specforge.analyze",
            json!({"test_results": named}),
        );
        assert_eq!(error["code"], "invalid_input", "{named:?}: {error}");
        assert_eq!(error["argument"], "test_results", "{named:?}: {error}");
    }
}
