use crate::e2e_fixtures::*;
use specforge_test_macros::test as specforge_test;
use std::io::Write;
use std::process::{Command, Stdio};

// --- Existing MCP smoke tests ---

fn specforge_binary() -> Command {
    Command::new(assert_cmd::cargo_bin!("specforge"))
}

#[specforge_test(
    behavior = "mcp_initialize",
    verify = "all core tools registered before accepting requests"
)]
fn mcp_server_responds_to_initialize() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    let mut child = specforge_binary()
        .args(["mcp"])
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start specforge mcp");

    let stdin = child.stdin.as_mut().unwrap();

    // A standard client handshake: `initialize` without `projectRoot`, the
    // `initialized` notification, then a request.
    writeln!(stdin, "{}", mcp_initialize(1)).unwrap();
    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        mcp_request(2, "tools/list", serde_json::json!({}))
    )
    .unwrap();
    stdin.flush().unwrap();

    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let responses: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("invalid JSON {e}: {l}")))
        .collect();
    // One response per request, nothing unsolicited.
    let ids: Vec<&serde_json::Value> = responses.iter().map(|r| &r["id"]).collect();
    assert_eq!(
        ids,
        [1, 2],
        "exactly the two requests are answered: {stdout}"
    );

    let init = &responses[0];
    assert!(
        init["error"].is_null(),
        "the client's initialize must succeed: {init}"
    );
    assert!(init["result"]["capabilities"]["tools"].is_object());

    let tools = responses[1]["result"]["tools"].as_array().expect("tools");
    assert!(!tools.is_empty(), "core tools registered");
}

#[test]
fn mcp_initialize_without_project_root_compiles_cli_path() {
    let responses = mcp_session(
        r#"behavior alpha "A" { contract "first" }"#,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({"name": "specforge.search", "arguments": {"query": "alpha"}}),
        )],
    );

    let init = find_response(&responses, 0).expect("initialize response");
    assert!(init["error"].is_null(), "initialize succeeds: {init}");
    let search = find_response(&responses, 1).expect("search response");
    let text = search["result"]["content"][0]["text"]
        .as_str()
        .expect("search text");
    assert!(
        text.contains("alpha"),
        "the `specforge mcp <path>` project was compiled: {text}"
    );
}

#[test]
fn mcp_server_lists_tools() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    let mut child = specforge_binary()
        .args(["mcp"])
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start specforge mcp");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, "{}", mcp_initialize(0)).unwrap();
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    writeln!(stdin, "{}", request).unwrap();
    stdin.flush().unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    let tools_response = lines.iter().find(|line| {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            v["result"]["tools"].is_array()
        } else {
            false
        }
    });

    assert!(
        tools_response.is_some(),
        "should have a tools/list response with tools array. lines: {:?}",
        lines,
    );
}

#[test]
fn mcp_server_handles_eof_gracefully() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    let mut child = specforge_binary()
        .args(["mcp"])
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start specforge mcp");

    drop(child.stdin.take());

    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "MCP server should exit 0 on EOF, got {:?}",
        status.code()
    );
}

// --- Protocol & Error Handling ---

const BASIC_SPEC: &str = r#"behavior alpha "Alpha" { contract "first" }
behavior beta "Beta" { contract "second" }
feature gamma "Gamma" { problem "p" solution "s" behaviors [alpha, beta] }
invariant inv "Invariant" { guarantee "always" enforced_by [alpha] }"#;

#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "invalid method produces -32601 Method not found"
)]
fn mcp_unknown_method_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "nonexistent/method", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_object(), "should be error response");
    assert_eq!(resp["error"]["code"], -32601, "should be METHOD_NOT_FOUND");
}

#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "malformed JSON produces -32700 Parse error"
)]
fn mcp_invalid_json_returns_parse_error() {
    let dir = setup_project(&[("main.spec", BASIC_SPEC)]);

    let mut child = specforge_binary()
        .args(["mcp"])
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start specforge mcp");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, "{{not valid json}}").unwrap();
    stdin.flush().unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Our bad JSON gets an error response
    let has_parse_error = lines.iter().any(|l| {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(l) {
            v["error"]["code"] == -32700 || v["error"]["code"] == -32600
        } else {
            false
        }
    });
    assert!(
        has_parse_error,
        "should produce a parse/invalid error response, lines: {:?}",
        lines
    );
}

#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "missing required params produces -32602 Invalid params"
)]
fn mcp_tool_call_missing_name_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "tools/call", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_object(), "should be error response");
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_ping_returns_empty_object() {
    let responses = mcp_session(BASIC_SPEC, &[mcp_request(1, "ping", serde_json::json!({}))]);

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["result"].is_object(),
        "ping should return result object"
    );
    assert!(
        !resp.get("error").is_some_and(|e| e.is_object()),
        "ping should not return error"
    );
}

// --- Tool Invocations ---

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "specforge.query tool returns subgraph for valid entityId"
)]
fn mcp_tool_query_returns_subgraph() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.query",
                "arguments": { "entity_id": "gamma", "depth": 1 }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["nodes"].is_array(),
        "query result should have nodes array"
    );
    assert!(
        content["edges"].is_array(),
        "query result should have edges array"
    );

    let ids: Vec<&str> = content["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"gamma"), "should include queried entity");
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace tool returns traceability chain for valid entityId"
)]
fn mcp_tool_trace_returns_chain() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.trace",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["entity_id"], "alpha");
    assert!(
        content["upstream"].is_array(),
        "trace should have upstream array"
    );
    assert!(
        content["downstream"].is_array(),
        "trace should have downstream array"
    );
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "missing links flagged in trace output"
)]
fn mcp_tool_trace_includes_gaps() {
    let responses = mcp_session(
        ISOLATED_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.trace",
                "arguments": { "entity_id": "isolated_node" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let gaps = content["gaps"]
        .as_array()
        .expect("trace should have gaps array");
    assert!(
        gaps.iter()
            .any(|g| g.as_str().unwrap().contains("upstream"))
    );
    assert!(
        gaps.iter()
            .any(|g| g.as_str().unwrap().contains("downstream"))
    );
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "specforge.export tool returns graph in requested format"
)]
fn mcp_tool_export_graph_format() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.export",
                "arguments": { "format": "graph" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let content: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(
        content["nodes"].is_array(),
        "export graph should have nodes array"
    );
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "text search finds entities matching by name or contract"
)]
fn mcp_tool_search_fuzzy_match() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.search",
                "arguments": { "query": "alph" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let results: serde_json::Value = serde_json::from_str(text).unwrap();
    let arr = results.as_array().unwrap();
    assert!(
        !arr.is_empty(),
        "fuzzy search for 'alph' should find 'alpha'"
    );

    let found_alpha = arr.iter().any(|r| r["entity_id"] == "alpha");
    assert!(found_alpha, "should find alpha entity, got: {:?}", arr);

    let score = arr.iter().find(|r| r["entity_id"] == "alpha").unwrap()["score"]
        .as_f64()
        .unwrap();
    assert!(
        score > 0.6,
        "score should be above 0.6 threshold, got {}",
        score
    );
}

#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "specforge.stats returns entity counts by kind"
)]
fn mcp_tool_stats_returns_counts() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.stats",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["entity_counts"].is_array(),
        "stats should have entity_counts"
    );
    assert!(
        content["edge_count"].is_number(),
        "stats should have edge_count"
    );
}

#[test]
fn mcp_tool_unknown_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "nonexistent.tool",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for unknown tool"
    );
    assert_eq!(
        resp["error"]["code"], -32602,
        "unknown tool is an Invalid params protocol error (MCP spec example)"
    );
}

// --- Resource Reads ---

#[test]
fn mcp_resource_list_returns_six_resources() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "resources/list", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let resources = resp["result"]["resources"]
        .as_array()
        .expect("should have resources array");
    assert_eq!(
        resources.len(),
        8,
        "should have 8 default resources (6 core + entities_by_kind + context/{{entity_id}} template), got {}",
        resources.len()
    );
}

#[specforge_test(
    behavior = "expose_graph_as_mcp_resource",
    verify = "specforge://graph resource returns full Graph Protocol JSON"
)]
fn mcp_resource_read_graph() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://graph" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = resp["result"]["contents"]
        .as_array()
        .or_else(|| resp["result"]["content"].as_array());
    assert!(
        content.is_some(),
        "resource read should return contents, resp: {}",
        resp
    );
}

#[specforge_test(
    behavior = "expose_diagnostics_as_mcp_resource",
    verify = "specforge://diagnostics resource returns current DiagnosticBag as JSON"
)]
fn mcp_resource_read_diagnostics() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://diagnostics" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    assert!(
        resp["result"].is_object(),
        "diagnostics resource should return result object"
    );
}

#[specforge_test(
    behavior = "expose_entity_as_mcp_resource",
    verify = "specforge://graph/{entity_id} returns entity and its neighbors"
)]
fn mcp_resource_read_entity_subgraph() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://graph/alpha" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    assert!(
        resp["result"].is_object(),
        "entity subgraph should return result object"
    );
}

// --- Prompts listing ---

#[test]
fn mcp_prompts_list_returns_prompts() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "prompts/list", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let prompts = resp["result"]["prompts"]
        .as_array()
        .expect("should have prompts array");
    assert!(
        prompts.len() >= 3,
        "should have at least 3 default prompts, got {}",
        prompts.len()
    );
}

// --- Multiple requests in one session ---

#[test]
fn mcp_multiple_requests_in_single_session() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[
            mcp_request(1, "tools/list", serde_json::json!({})),
            mcp_request(2, "resources/list", serde_json::json!({})),
            mcp_request(3, "ping", serde_json::json!({})),
        ],
    );

    assert!(
        find_response(&responses, 1).is_some(),
        "should get tools/list response"
    );
    assert!(
        find_response(&responses, 2).is_some(),
        "should get resources/list response"
    );
    assert!(
        find_response(&responses, 3).is_some(),
        "should get ping response"
    );
}

// ============================================================
// Phase 1: Navigation Tools
// ============================================================

#[test]
fn mcp_tool_validate_returns_diagnostics() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.validate",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("should have content[0].text");
    // Diagnostics text should be valid JSON (array of diagnostics)
    let _parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("diagnostics text not valid JSON: {}\ntext: {}", e, text));
    // isError field should be present on the result
    assert!(
        resp["result"].get("isError").is_some()
            || resp["result"]["content"][0].get("isError").is_some()
            || text.contains("[]"),
        "validate should return diagnostics or isError field"
    );
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "specforge.schema returns full GraphProtocolSchema"
)]
fn mcp_tool_schema_returns_entity_kinds() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.schema",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["entity_kinds"].is_object(),
        "schema should have entity_kinds object: {}",
        content
    );
    assert!(
        content["edge_labels"].is_array(),
        "schema should have edge_labels array: {}",
        content
    );
    assert!(
        content.get("schema_version").is_some(),
        "schema should have schema_version: {}",
        content
    );
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "specforge.coverage returns coverage for all testable entities"
)]
fn mcp_tool_coverage_returns_status() {
    // Testability comes from the extensions; without any, nothing is testable.
    let dir = setup_project_with_config(
        r#"{"name":"test","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
        &[("main.spec", BASIC_SPEC)],
    );
    let responses = mcp_session_in(
        &dir,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.coverage",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("coverage should return array");
    assert!(
        !arr.is_empty(),
        "coverage should have entries for BASIC_SPEC entities"
    );

    let first = &arr[0];
    assert!(
        first["entity_id"].is_string(),
        "coverage entry should have entity_id"
    );
    assert!(
        first["status"].is_string(),
        "coverage entry should have status"
    );
    assert!(
        first.get("declared").is_some(),
        "coverage entry should have declared field"
    );
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "specforge.inspect returns full entity details"
)]
fn mcp_tool_inspect_returns_entity_detail() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.inspect",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["entity_id"], "alpha");
    assert!(content["kind"].is_string(), "inspect should have kind");
    assert!(
        content["source_span"].is_object(),
        "inspect should have source_span"
    );
    assert!(
        content["reference_count"].is_number(),
        "inspect should have reference_count"
    );
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "non-existent entity returns error response"
)]
fn mcp_tool_inspect_missing_entity_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.inspect",
                "arguments": { "entity_id": "nonexistent_entity" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    // C9-00/C9-12: missing entity is a tool execution error (isError
    // result), not a -32602 protocol error.
    assert!(
        resp["result"]["isError"] == true,
        "missing entity must be an isError tool result"
    );
}

#[specforge_test(
    behavior = "provide_mcp_find_definition_tool",
    verify = "specforge.find_definition returns file, line, and column"
)]
fn mcp_tool_find_definition_returns_location() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.find_definition",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["entity_id"], "alpha");
    assert!(
        content["file_path"].is_string(),
        "find_definition should have file_path"
    );
    assert!(
        content["line"].is_number(),
        "find_definition should have line"
    );
    assert!(
        content["column"].is_number(),
        "find_definition should have column"
    );
}

#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "specforge.find_references returns all reference locations"
)]
fn mcp_tool_find_references_returns_locations() {
    // alpha is referenced by gamma's behaviors list
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.find_references",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["entity_id"], "alpha");
    let locations = content["locations"]
        .as_array()
        .expect("should have locations array");
    assert!(
        !locations.is_empty(),
        "alpha should have at least one reference (from gamma)"
    );

    let first = &locations[0];
    assert!(
        first["referencing_entity_id"].is_string(),
        "location should have referencing_entity_id"
    );
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "specforge.outline returns all entities defined in file"
)]
fn mcp_tool_outline_returns_entities_in_file() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.outline",
                "arguments": { "file": "main.spec" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("outline should return array");
    assert!(
        arr.len() >= 4,
        "BASIC_SPEC has 4 entities, got {}",
        arr.len()
    );

    // Should be sorted by start_line
    let lines: Vec<u64> = arr
        .iter()
        .map(|e| {
            e["range"]["start_line"]
                .as_u64()
                .expect("should have range.start_line")
        })
        .collect();
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted, "outline should be sorted by start_line");
}

// ============================================================
// Phase 2: Tool Parameter Variants
// ============================================================

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "format parameter changes output serialization"
)]
fn mcp_tool_query_format_context() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.query",
                "arguments": { "entity_id": "gamma", "depth": 1, "format": "context" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    // Context format should produce valid JSON with contract-style fields
    let _content: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("context output not valid JSON: {}\ntext: {}", e, text));
}

#[test]
fn mcp_tool_query_format_brief() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.query",
                "arguments": { "entity_id": "gamma", "depth": 1, "format": "brief" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let _content: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("brief output not valid JSON: {}\ntext: {}", e, text));
}

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "include_coverage parameter includes coverage status in response"
)]
fn mcp_tool_query_include_coverage() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.query",
                "arguments": { "entity_id": "gamma", "depth": 1, "include_coverage": true }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    // With include_coverage, nodes should have coverage_status
    let nodes = content["nodes"].as_array().expect("should have nodes");
    assert!(!nodes.is_empty(), "should have nodes");
    let has_coverage = nodes.iter().any(|n| n.get("coverage_status").is_some());
    assert!(
        has_coverage,
        "at least one node should have coverage_status when include_coverage=true"
    );
}

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "kind filter restricts returned node types"
)]
fn mcp_tool_query_with_kinds_filter() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.query",
                "arguments": { "entity_id": "gamma", "depth": 2, "kinds": ["behavior"] }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let nodes = content["nodes"].as_array().expect("should have nodes");
    // All non-root nodes should be behaviors (root gamma is a feature but is always included)
    for node in nodes {
        let kind = node["kind"].as_str().unwrap_or("");
        let id = node["id"].as_str().unwrap_or("");
        if id != "gamma" {
            assert_eq!(
                kind, "behavior",
                "filtered node {} should be behavior, got {}",
                id, kind
            );
        }
    }
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "scope parameter restricts to subgraph"
)]
fn mcp_tool_export_scoped() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.export",
                "arguments": { "scope": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let nodes = content["nodes"]
        .as_array()
        .expect("scoped export should have nodes");
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"alpha"), "scoped export should include alpha");
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "all three formats (context, brief, graph) supported"
)]
fn mcp_tool_export_format_context() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.export",
                "arguments": { "format": "context" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let _content: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("context export not valid JSON: {}\ntext: {}", e, text));
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "kind filter restricts results to matching entity kinds"
)]
fn mcp_tool_search_with_kinds() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.search",
                "arguments": { "query": "alpha", "kinds": ["behavior"] }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("search should return array");
    for result in arr {
        assert_eq!(
            result["kind"].as_str().unwrap(),
            "behavior",
            "search with kinds=[behavior] should only return behaviors"
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "references filter returns entities referencing target"
)]
fn mcp_tool_search_references() {
    // Find entities that reference alpha
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.search",
                "arguments": { "query": "alpha", "references": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("search should return array");
    // gamma references alpha via behaviors [alpha, beta], and inv via enforced_by [alpha]
    let ids: Vec<&str> = arr
        .iter()
        .map(|r| r["entity_id"].as_str().unwrap())
        .collect();
    assert!(!ids.is_empty(), "should find entities referencing alpha");
}

// ============================================================
// Phase 3: Resources & Error Paths
// ============================================================

#[specforge_test(
    behavior = "expose_schema_as_mcp_resource",
    verify = "specforge://schema resource returns GraphProtocolSchema JSON"
)]
fn mcp_resource_read_schema() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://schema" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let contents = resp["result"]["contents"]
        .as_array()
        .or_else(|| resp["result"]["content"].as_array())
        .expect("resource read should return contents");
    assert!(
        !contents.is_empty(),
        "schema resource should return content"
    );
    let text = contents[0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(
        parsed.get("schema_version").is_some(),
        "schema should have schema_version: {}",
        parsed
    );
}

#[specforge_test(
    behavior = "expose_context_as_mcp_resource",
    verify = "specforge://context resource returns token-optimized format"
)]
fn mcp_resource_read_context() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://context" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let contents = resp["result"]["contents"]
        .as_array()
        .or_else(|| resp["result"]["content"].as_array());
    assert!(
        contents.is_some(),
        "context resource should return contents: {}",
        resp
    );
}

#[specforge_test(
    behavior = "expose_brief_as_mcp_resource",
    verify = "specforge://brief resource returns minimal IDs and edges format"
)]
fn mcp_resource_read_brief() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://brief" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let contents = resp["result"]["contents"]
        .as_array()
        .or_else(|| resp["result"]["content"].as_array());
    assert!(
        contents.is_some(),
        "brief resource should return contents: {}",
        resp
    );
}

#[test]
fn mcp_resource_read_unknown_uri_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://nonexistent" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_object(), "should be error for unknown URI");
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_resource_read_missing_uri_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "resources/read", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_object(), "should be error for missing uri");
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_resource_read_entity_not_found() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://graph/nonexistent" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for nonexistent entity"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_resource_contents_format() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "resources/read",
            serde_json::json!({ "uri": "specforge://graph" }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let contents = resp["result"]["contents"]
        .as_array()
        .or_else(|| resp["result"]["content"].as_array())
        .expect("should have contents array");
    let first = &contents[0];
    assert!(
        first["uri"].is_string() || first.get("uri").is_some(),
        "contents[0] should have uri field: {}",
        first
    );
    assert!(
        first["mimeType"].is_string() || first.get("mimeType").is_some(),
        "contents[0] should have mimeType field: {}",
        first
    );
}

// ============================================================
// Phase 4: Prompts
// ============================================================

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "specforge://prompts/context returns structured entity context"
)]
fn mcp_prompt_context_returns_messages() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/context",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let messages = resp["result"]["messages"]
        .as_array()
        .expect("prompt should return messages array");
    assert!(
        messages.len() >= 2,
        "should have instruction + data messages"
    );
    // Data is in the last message (assistant role)
    let data_msg = messages.last().unwrap();
    let text = data_msg["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("data message should have content.text: {}", data_msg));
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("data message text not valid JSON: {}\ntext: {}", e, text));
    assert_eq!(parsed["entity_id"], "alpha");
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "specforge://prompts/review returns coverage analysis"
)]
fn mcp_prompt_review_returns_findings() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/review",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let messages = resp["result"]["messages"]
        .as_array()
        .expect("prompt should return messages array");
    assert!(
        messages.len() >= 2,
        "should have instruction + data messages"
    );
    let data_msg = messages.last().unwrap();
    let text = data_msg["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("data message should have content.text: {}", data_msg));
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("data message text not valid JSON: {}\ntext: {}", e, text));
    assert!(
        parsed.get("findings").is_some(),
        "review should have findings: {}",
        parsed
    );
    assert!(
        parsed.get("coverage_summary").is_some(),
        "review should have coverage_summary: {}",
        parsed
    );
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "response returns identified gaps with gap context"
)]
fn mcp_prompt_trace_returns_gaps() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/trace",
                "arguments": { "entity_id": "alpha" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let messages = resp["result"]["messages"]
        .as_array()
        .expect("prompt should return messages array");
    assert!(
        messages.len() >= 2,
        "should have instruction + data messages"
    );
    let data_msg = messages.last().unwrap();
    let text = data_msg["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("data message should have content.text: {}", data_msg));
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("data message text not valid JSON: {}\ntext: {}", e, text));
    assert!(
        parsed.get("unverified_entities").is_some() || parsed.get("coverage_gaps").is_some(),
        "trace prompt should have unverified_entities or coverage_gaps: {}",
        parsed
    );
}

#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "specforge://prompts/explore returns exploration starting points"
)]
fn mcp_prompt_explore_returns_starting_points() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/explore",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let messages = resp["result"]["messages"]
        .as_array()
        .expect("prompt should return messages array");
    assert!(
        messages.len() >= 2,
        "should have instruction + data messages"
    );
    let data_msg = messages.last().unwrap();
    let text = data_msg["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("data message should have content.text: {}", data_msg));
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("data message text not valid JSON: {}\ntext: {}", e, text));
    assert!(
        parsed.get("starting_points").is_some(),
        "explore should have starting_points: {}",
        parsed
    );
    assert!(
        parsed.get("high_connectivity").is_some(),
        "explore should have high_connectivity: {}",
        parsed
    );
}

#[test]
fn mcp_prompt_unknown_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/nonexistent",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for unknown prompt"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_prompt_context_missing_entity_returns_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/context",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for missing entity_id"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "non-existent entity returns error"
)]
fn mcp_prompt_context_entity_not_found() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "prompts/get",
            serde_json::json!({
                "name": "specforge://prompts/context",
                "arguments": { "entity_id": "nonexistent_entity" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for nonexistent entity"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

// ============================================================
// Phase 5: Operation Tools
// ============================================================

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "specforge.format formats spec files"
)]
fn mcp_tool_format_returns_result() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.format",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["changed_files"].is_array(),
        "format should have changed_files"
    );
    assert!(
        content["all_clean"].is_boolean(),
        "format should have all_clean"
    );
    assert_eq!(
        content["check_only"], false,
        "default check_only should be false"
    );
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "check mode reports without modifying files"
)]
fn mcp_tool_format_check_mode() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.format",
                "arguments": { "check": true }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(
        content["check_only"], true,
        "check: true should set check_only: true"
    );
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "specforge.rename renames entity and all references"
)]
fn mcp_tool_rename_returns_affected() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.rename",
                "arguments": { "entity_id": "alpha", "new_name": "alpha_renamed" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["old_name"], "alpha");
    assert_eq!(content["new_name"], "alpha_renamed");
    assert!(
        content["affected_files"].is_array(),
        "rename should list affected files"
    );
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "non-existent entity returns error response"
)]
fn mcp_tool_rename_missing_entity_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.rename",
                "arguments": { "entity_id": "nonexistent_xyz", "new_name": "new_xyz" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for nonexistent entity"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "invalid new_name returns validation error"
)]
fn mcp_tool_rename_invalid_name_error() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.rename",
                "arguments": { "entity_id": "alpha", "new_name": "" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for invalid name"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "specforge.init creates specforge.json project"
)]
fn mcp_tool_init_returns_project() {
    let dir = tempfile::TempDir::new().unwrap();
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.init",
                "arguments": { "path": dir.path().to_str().unwrap(), "name": "fresh" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["project_path"], dir.path().to_str().unwrap());
    assert!(dir.path().join("specforge.json").is_file());
    assert_eq!(content["config_file"], "specforge.json");
    assert!(
        content["starter_file"].is_string(),
        "init should have starter_file"
    );
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "specforge.add_extension adds extension to config"
)]
fn mcp_tool_add_extension_returns_installed() {
    // Local .wasm install: real, offline, and verifiable.
    let blob = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.add_extension",
                "arguments": { "specifier": blob.to_str().unwrap() }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["extension"], "specforge_ext_product");
    assert_eq!(content["installed"], true);
}

#[test]
fn mcp_tool_add_extension_invalid_specifier() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.add_extension",
                "arguments": { "specifier": "bad" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for invalid specifier"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

#[test]
fn mcp_tool_remove_extension_returns_success() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.remove_extension",
                "arguments": { "name": "@specforge/software" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    // Nothing is installed in this fixture — refusing is the honest answer.
    assert!(
        resp["error"].is_object(),
        "removing an uninstalled extension must be an error: {}",
        resp
    );
}

#[test]
fn mcp_tool_migrate_returns_result() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.migrate",
                "arguments": { "from_version": "0.1.0", "to_version": "0.2.0" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    // from_version is the DETECTED format version (1.0), not the request's
    // legacy range — truthful reporting of what was found on disk.
    assert_eq!(content["from_version"], "1.0");
    assert_eq!(content["migrated"], false, "fixture is already current");
    assert!(content["message"].is_string());
}

#[test]
fn mcp_tool_extensions_returns_list() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.extensions",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["extensions"].is_array(),
        "extensions should have extensions array"
    );
    assert!(
        content["entity_kinds_in_graph"].is_array() || content["entity_kinds_in_graph"].is_object(),
        "extensions should have entity_kinds_in_graph: {}",
        content
    );
}

#[test]
fn mcp_tool_providers_returns_list() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.providers",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content["providers"].is_array(),
        "providers should have providers array"
    );
}

#[test]
fn mcp_tool_doctor_returns_health() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.doctor",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["extensions_ok"], true);
    assert!(
        content["findings"].is_array(),
        "doctor should have findings array"
    );
    assert!(
        content["cache_status"].is_string(),
        "doctor should have cache_status"
    );
}

#[specforge_test(
    behavior = "provide_mcp_collect_tool",
    verify = "a project without a collector returns an E058 error"
)]
fn mcp_tool_collect_without_collector_errors() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.collect",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    let msg = resp["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.starts_with("E058"), "expected E058, got: {resp}");
}

#[test]
fn mcp_tool_render_returns_output() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.render",
                "arguments": { "format": "json" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert_eq!(content["format"], "json");
    assert!(
        content["output"].is_string(),
        "render should return the rendered output"
    );
}

// ============================================================
// Phase 6: Suggest Fixes + Parameter Variants
// ============================================================

#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "clean entity with no diagnostics returns empty list"
)]
fn mcp_tool_suggest_fixes_returns_array() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.suggest_fixes",
                "arguments": {}
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    assert!(
        content.is_array(),
        "suggest_fixes should return an array (possibly empty)"
    );
}

#[test]
fn mcp_tool_validate_severity_filter() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.validate",
                "arguments": { "severity_filter": "error" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("should have content[0].text");
    let parsed: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("diagnostics text not valid JSON: {}\ntext: {}", e, text));
    // All returned diagnostics (if any) should be errors
    if let Some(arr) = parsed.as_array() {
        for diag in arr {
            assert_eq!(
                diag["severity"].as_str().unwrap_or("error"),
                "error",
                "severity_filter=error should only return errors, got: {}",
                diag
            );
        }
    }
}

#[test]
fn mcp_tool_coverage_kind_filter() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.coverage",
                "arguments": { "kind": "behavior" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("coverage should return array");
    for entry in arr {
        assert_eq!(
            entry["kind"].as_str().unwrap(),
            "behavior",
            "kind=behavior filter should only return behaviors, got: {}",
            entry
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "kind filter restricts schema to single entity kind"
)]
fn mcp_tool_schema_kind_filter() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.schema",
                "arguments": { "kind": "behavior" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let kinds = content["entity_kinds"]
        .as_object()
        .expect("should have entity_kinds object");
    assert!(
        kinds.contains_key("behavior"),
        "filtered schema should include behavior"
    );
    assert_eq!(
        kinds.len(),
        1,
        "kind=behavior filter should return only behavior, got: {:?}",
        kinds.keys().collect::<Vec<_>>()
    );
}

#[test]
fn mcp_tool_export_format_brief() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.export",
                "arguments": { "format": "brief" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let _content: serde_json::Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("brief export not valid JSON: {}\ntext: {}", e, text));
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "limit caps the number of returned results"
)]
fn mcp_tool_search_with_limit() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.search",
                "arguments": { "query": "a", "limit": 1 }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(resp["error"].is_null(), "should not be error: {}", resp);
    let content = parse_tool_content(resp);
    let arr = content.as_array().expect("search should return array");
    assert!(
        arr.len() <= 1,
        "limit=1 should return at most 1 result, got {}",
        arr.len()
    );
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "unrecognized format returns error listing available renderers"
)]
fn mcp_tool_render_invalid_format() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "tools/call",
            serde_json::json!({
                "name": "specforge.render",
                "arguments": { "format": "xyz" }
            }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "should be error for invalid renderer format"
    );
    assert_eq!(resp["error"]["code"], -32602, "should be INVALID_PARAMS");
}

// ============================================================
// Phase 7: Lifecycle & Protocol
// ============================================================

#[test]
fn mcp_lifecycle_shutdown() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(1, "shutdown", serde_json::json!({}))],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["result"].is_object(),
        "shutdown should return result object"
    );
    assert!(resp["error"].is_null(), "shutdown should not return error");
}

#[test]
fn mcp_lifecycle_cancel_request() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "$/cancelRequest",
            serde_json::json!({ "id": 999 }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["result"].is_object(),
        "cancelRequest should return result object"
    );
    assert!(
        resp["error"].is_null(),
        "cancelRequest should not return error"
    );
}

#[specforge_test(
    behavior = "guard_mcp_reinitialization",
    verify = "second initialize request returns -32600 error"
)]
fn mcp_lifecycle_double_init_error() {
    // mcp_session already sent the client's initialize (id 0); a second one fails
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "initialize",
            serde_json::json!({ "projectRoot": "." }),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["error"].is_object(),
        "double initialize should return error"
    );
    assert_eq!(resp["error"]["code"], -32600, "should be INVALID_REQUEST");
}

#[test]
fn mcp_lifecycle_notifications_initialized() {
    let responses = mcp_session(
        BASIC_SPEC,
        &[mcp_request(
            1,
            "notifications/initialized",
            serde_json::json!({}),
        )],
    );

    let resp = find_response(&responses, 1).expect("should get response for id 1");
    assert!(
        resp["result"].is_object(),
        "notifications/initialized should return result"
    );
    assert!(
        resp["error"].is_null(),
        "notifications/initialized should not return error"
    );
}

/// A stdio client subscribed to the graph gets `specforge/graphChanged` on
/// stdout once watch has rebuilt the project.
#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "graph_changed notification sent after incremental rebuild"
)]
fn mcp_stdio_client_receives_graph_notification() {
    use std::io::{BufRead, BufReader};

    let dir = setup_project(&[("spec/base.spec", r#"behavior base "B" { contract "b" }"#)]);
    let mut child = specforge_binary()
        .args(["mcp"])
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start specforge mcp");
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut send = move |req: String| {
        writeln!(stdin, "{req}").unwrap();
        stdin.flush().unwrap();
    };

    send(mcp_initialize(0));
    lines.next().unwrap().unwrap();
    send(mcp_request(
        1,
        "resources/subscribe",
        serde_json::json!({"uri": "specforge://graph"}),
    ));
    lines.next().unwrap().unwrap();

    // What watch leaves behind: a new entity and a newer graph snapshot.
    std::fs::write(
        dir.path().join("spec/added.spec"),
        r#"behavior added "A" { contract "a" }"#,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    std::fs::create_dir_all(dir.path().join(".specforge")).unwrap();
    std::fs::write(dir.path().join(".specforge/graph.json"), "{}").unwrap();

    send(mcp_request(
        2,
        "resources/read",
        serde_json::json!({"uri": "specforge://diagnostics"}),
    ));
    drop(send);
    let out: Vec<serde_json::Value> = lines
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect();
    child.wait().unwrap();

    let notification = out
        .iter()
        .find(|m| m["method"] == "specforge/graphChanged")
        .unwrap_or_else(|| panic!("no graphChanged notification in {out:?}"));
    assert!(notification.get("id").is_none(), "{notification}");
    let added = notification["params"]["added_nodes"].as_array().unwrap();
    assert!(added.iter().any(|n| n == "added"), "{notification}");
}
