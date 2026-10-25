//! A source-code path an MCP request names is read by one reader whatever the argument (ADR 0253):
//! infer_session's `source_file`, find_spec_for_source's `file_path`, the infer prompt's `file:` scope.

use crate::support::*;
use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test_macros::test as specforge_test;

const LEAVES: &str = "a source file is named from the project root";

/// A project with `src/lib.rs`, anchored to `alpha`.
fn served() -> Served {
    TestProject::new()
        .file("test.spec", "behavior alpha \"Alpha\" {\n}\n")
        .file("src/lib.rs", "pub fn alpha() {}\n")
        .file(
            "specforge-anchors.json",
            json!({"version": 1, "anchors": [{"entity_id": "alpha", "file": "src/lib.rs",
                   "line": 1, "symbol_name": "alpha", "item_kind": "fn", "scanner": "manual"}]})
            .to_string(),
        )
        .serve(&[TestExtension::software()])
}

fn marked(server: &mut McpServer, source_file: &str) -> Value {
    call_tool(
        server,
        "specforge.infer_session",
        json!({"action": "mark_analyzed", "source_file": source_file}),
    )
}

fn found(server: &mut McpServer, file_path: &str) -> Value {
    call_tool(
        server,
        "specforge.find_spec_for_source",
        json!({"file_path": file_path}),
    )
}

fn scoped(server: &mut McpServer, spelling: &str) -> Value {
    get_prompt(
        server,
        "specforge://prompts/infer",
        json!({"scope": format!("file:{spelling}")}),
    )
}

/// `reply` refused on `argument` in the reader's sentence for `named`.
fn assert_leaves(error: &Value, argument: &str, named: &str) {
    assert_eq!(error["code"], "invalid_input", "{named}: {error}");
    assert_eq!(error["argument"], argument, "{named}: {error}");
    assert_eq!(
        error["message"],
        format!("'{named}' lies outside the project root: {LEAVES}"),
        "{error}"
    );
}

#[ignore = "flipped by T4"]
#[specforge_test(
    invariant = "mcp_served_project_consistency",
    verify = "a path an MCP request names resolves inside the served project, whatever its spelling"
)]
#[specforge_test(
    behavior = "read_a_named_source_path",
    verify = "infer_session, find_spec_for_source and the infer file scope read one table of spellings alike"
)]
fn every_source_path_argument_reads_one_table_of_spellings() {
    let mut server = served();
    let root = server.root().to_path_buf();
    let absolute = root.join("src/lib.rs").display().to_string();
    for spelling in [
        "src/lib.rs",
        "./src//lib.rs",
        "src\\lib.rs",
        absolute.as_str(),
        "src/../src/lib.rs",
    ] {
        let reply = marked(&mut server, spelling);
        assert_eq!(reply["result"]["isError"], false, "{spelling}: {reply}");
        let tool = tool(
            &mut server,
            "specforge.find_spec_for_source",
            json!({"file_path": spelling}),
        );
        assert_eq!(tool["match_mode"], "exact", "{spelling}: {tool}");
        assert_eq!(tool["entities"][0]["entity_id"], "alpha", "{spelling}");
        let prompt = prompt_payload(&scoped(&mut server, spelling));
        assert_eq!(prompt["match_mode"], "exact", "{spelling}: {prompt}");
    }
    let other = tempfile::TempDir::new().unwrap();
    let elsewhere = other.path().join("lib.rs").display().to_string();
    let beyond = format!("{}/../o/lib.rs", root.display());
    for named in ["../o/lib.rs", elsewhere.as_str(), beyond.as_str()] {
        assert_leaves(
            &crate::tool_errors::mcp_error(&marked(&mut server, named)),
            "source_file",
            named,
        );
        assert_leaves(
            &crate::tool_errors::mcp_error(&found(&mut server, named)),
            "file_path",
            named,
        );
        let prompt = scoped(&mut server, named);
        assert_eq!(prompt["error"]["code"], -32602, "{named}: {prompt}");
        assert_eq!(
            prompt["error"]["data"]["argument"], "scope",
            "{named}: {prompt}"
        );
        assert_eq!(
            prompt["error"]["message"],
            format!("'{named}' lies outside the project root: {LEAVES}"),
            "{prompt}"
        );
    }
}

#[ignore = "flipped by T4"]
#[specforge_test(
    behavior = "provide_mcp_find_spec_for_source_tool",
    verify = "a file_path outside the project root is refused as invalid_input on file_path"
)]
fn a_file_path_outside_the_project_is_refused_on_file_path() {
    let mut server = served();
    let other = tempfile::TempDir::new().unwrap();
    let hosts = other.path().join("hosts").display().to_string();
    let reply = found(&mut server, &hosts);
    let error = crate::tool_errors::mcp_error(&reply);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "file_path", "{error}");
}

#[ignore = "flipped by T5"]
#[test]
fn render_names_a_refused_out_dir_as_the_call_spelled_it() {
    let mut server = served();
    let reply = call_tool(
        &mut server,
        "specforge.render",
        json!({"format": "graph", "out_dir": "../escape_out"}),
    );
    let error = crate::tool_errors::mcp_error(&reply);
    let message = error["message"].as_str().unwrap();
    assert!(
        message.starts_with("'../escape_out' resolves to "),
        "{message}"
    );
}

#[ignore = "flipped by T6"]
#[test]
fn a_call_targets_path_outside_is_refused_in_the_containments_sentence() {
    let mut server = served();
    let reply = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": "../outside"}),
    );
    let text = reply.to_string();
    assert!(text.contains("'../outside' resolves to "), "{text}");
    assert!(text.contains(", which bounds the request"), "{text}");
}
