//! Protocol revision negotiation and what the negotiated revision changes
//! on the wire: JSON-RPC batches (2025-03-26 only), `structuredContent`
//! (2025-06-18 on), and resource templates.

use crate::support::*;
use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// A server initialized with `version` as the client's protocol version
/// (none when `None`), and the version it answered with.
fn negotiated(version: Option<&str>) -> (McpServer, Value) {
    let mut server = McpServer::new();
    let params = match version {
        Some(v) => json!({"protocolVersion": v, "capabilities": {}}),
        None => json!({}),
    };
    let resp = call(&mut server, "initialize", params);
    let answered = resp["result"]["protocolVersion"].clone();
    (server, answered)
}

fn send_raw(server: &mut McpServer, input: &str) -> Option<Value> {
    server
        .handle_message(input)
        .map(|resp| serde_json::from_str(&resp).unwrap())
}

#[specforge_test(
    behavior = "mcp_initialize",
    verify = "answers with the client's protocol version when it supports it"
)]
fn initialize_echoes_a_supported_protocol_version() {
    for version in ["2025-11-25", "2025-06-18", "2025-03-26"] {
        let (_, answered) = negotiated(Some(version));
        assert_eq!(answered, version);
    }
}

#[specforge_test(
    behavior = "mcp_initialize",
    verify = "answers an unsupported or missing protocol version with 2025-11-25"
)]
fn initialize_answers_other_versions_with_the_latest() {
    for version in [Some("2024-11-05"), Some("2099-01-01"), None] {
        let (_, answered) = negotiated(version);
        assert_eq!(answered, "2025-11-25", "client asked for {version:?}");
    }
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a 2025-03-26 session answers a batch with the response to each request"
)]
fn batch_on_2025_03_26_gets_each_response() {
    let (mut server, _) = negotiated(Some("2025-03-26"));
    let batch = json!([
        request(1, "ping", json!({})),
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2},
        request(3, "tools/call", json!({"name": "specforge.stats", "arguments": {}})),
    ]);

    let resp = send_raw(&mut server, &batch.to_string()).expect("a batch with requests");
    let responses = resp.as_array().expect("an array of responses");
    // One response per request, none for the notification.
    assert_eq!(responses.len(), 3, "{resp}");
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"], json!({}));
    // An invalid member is answered with its own error.
    assert_eq!(responses[1]["id"], 2);
    assert_eq!(responses[1]["error"]["code"], -32600);
    assert_eq!(responses[2]["id"], 3);
    assert!(responses[2]["result"]["content"].is_array(), "{resp}");
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a batch of notifications gets no response"
)]
fn batch_of_notifications_gets_no_response() {
    let (mut server, _) = negotiated(Some("2025-03-26"));
    let batch = json!([
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 9}},
    ]);
    assert_eq!(send_raw(&mut server, &batch.to_string()), None);
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "an empty batch is an invalid request"
)]
fn empty_batch_is_invalid() {
    let (mut server, _) = negotiated(Some("2025-03-26"));
    let resp = send_raw(&mut server, "[]").unwrap();
    assert_eq!(resp["error"]["code"], -32600, "{resp}");
    assert_eq!(resp.get("id"), Some(&Value::Null), "{resp}");
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a session on a later revision rejects a batch with -32600"
)]
fn batch_on_later_revisions_is_rejected() {
    let batch = json!([request(1, "ping", json!({})), request(2, "ping", json!({}))]).to_string();
    for version in ["2025-06-18", "2025-11-25"] {
        let (mut server, _) = negotiated(Some(version));
        let resp = send_raw(&mut server, &batch).unwrap();
        assert_eq!(resp["error"]["code"], -32600, "{version}: {resp}");
        assert_eq!(resp.get("id"), Some(&Value::Null), "{resp}");
        // The session carries on.
        assert_eq!(call(&mut server, "ping", json!({}))["result"], json!({}));
    }
    // Before initialize no revision is negotiated, so there is no batching.
    let resp = send_raw(&mut McpServer::new(), &batch).unwrap();
    assert_eq!(resp["error"]["code"], -32600, "{resp}");
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "tool results carry an object payload as structuredContent from 2025-06-18"
)]
fn structured_content_from_2025_06_18() {
    for version in ["2025-06-18", "2025-11-25"] {
        let (mut server, _) = negotiated(Some(version));
        let stats = call(
            &mut server,
            "tools/call",
            json!({"name": "specforge.stats", "arguments": {}}),
        );
        let text: Value =
            serde_json::from_str(stats["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(text.is_object());
        assert_eq!(stats["result"]["structuredContent"], text, "{version}");

        // An array payload is not an object: text only.
        let list = call(
            &mut server,
            "tools/call",
            json!({"name": "specforge.list", "arguments": {}}),
        );
        assert!(list["result"]["content"][0]["text"].is_string(), "{list}");
        assert!(list["result"].get("structuredContent").is_none(), "{list}");
    }
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a 2025-03-26 session gets no structuredContent"
)]
fn no_structured_content_on_2025_03_26() {
    let (mut server, _) = negotiated(Some("2025-03-26"));
    let stats = call(
        &mut server,
        "tools/call",
        json!({"name": "specforge.stats", "arguments": {}}),
    );
    assert!(stats["result"]["content"][0]["text"].is_string(), "{stats}");
    assert!(
        stats["result"].get("structuredContent").is_none(),
        "{stats}"
    );
}

#[specforge_test(
    behavior = "list_mcp_resources",
    verify = "templated resources are listed by resources/templates/list, not resources/list"
)]
fn templated_resources_are_resource_templates() {
    let (mut server, _) = negotiated(Some("2025-11-25"));

    let listed = call(&mut server, "resources/list", json!({}));
    let uris: Vec<&str> = listed["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.iter().all(|u| !u.contains('{')), "{uris:?}");
    assert!(uris.contains(&"specforge://graph"), "{uris:?}");

    let templates = call(&mut server, "resources/templates/list", json!({}));
    let templates = templates["result"]["resourceTemplates"].as_array().unwrap();
    let uri_templates: Vec<&str> = templates
        .iter()
        .map(|t| t["uriTemplate"].as_str().unwrap())
        .collect();
    assert_eq!(
        uri_templates,
        [
            "specforge://context/{entity_id}",
            "specforge://graph/{entity_id}",
            "specforge://entities/{kind}",
        ]
    );
    for template in templates {
        assert!(template["name"].is_string(), "{template}");
        assert_eq!(template["mimeType"], "application/json", "{template}");
    }
}

/// A 2025-06-18 server over a throwaway project with the software and
/// testing extensions, holding behavior `alpha`, feature `beta`, a Rust
/// source file and an inference manifest.
/// A project `@specforge/cargo-test` collects from, with the report an
/// earlier `cargo test` wrote.
fn collect_project() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        json!({"name": "c", "version": "0.1.0",
            "extensions": ["@specforge/software", "@specforge/testing", "@specforge/cargo-test"]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior alpha \"Alpha\" {\n  verify unit \"works\"\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("Cargo.toml"), "").unwrap();
    std::fs::create_dir_all(root.join("target/specforge")).unwrap();
    std::fs::write(
        root.join("target/specforge/t.json"),
        json!({"entries": [{"entity_id": "alpha", "test_name": "works", "status": "pass"}]})
            .to_string(),
    )
    .unwrap();
    dir
}

fn structured_server() -> (McpServer, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("specforge.json"),
        json!({"name": "t", "version": "0.1.0",
            "extensions": ["@specforge/software", "@specforge/testing"]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("test.spec"),
        "behavior alpha \"Alpha\" {\n  category command\n  contract \"MUST work\"\n  verify unit \"works\"\n}\n\nfeature beta \"Beta\" {\n  behaviors [alpha]\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    std::fs::write(
        root.join("specforge-infer.json"),
        json!({"version": 1, "source_roots": ["src"]}).to_string(),
    )
    .unwrap();
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"protocolVersion": "2025-06-18", "projectRoot": root.to_str().unwrap()}),
    );
    (server, dir)
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "each core tool with an object result declares an outputSchema its structured results conform to"
)]
fn structured_results_conform_to_each_tool_output_schema() {
    let (mut server, _dir) = structured_server();
    // init refuses a path inside the served project.
    let elsewhere = tempfile::TempDir::new().unwrap();
    let new_project = elsewhere.path().join("new");
    // collect reads the report an earlier `cargo test` wrote; nothing runs.
    let collected = collect_project();
    let probes = [
        ("specforge.query", json!({"entity_id": "alpha"})),
        (
            "specforge.query",
            json!({"entity_id": "alpha", "format": "brief"}),
        ),
        ("specforge.analyze", json!({"use_cached": true})),
        ("specforge.trace", json!({"entity_id": "alpha"})),
        (
            "specforge.trace",
            json!({"plan": {"entries": [{"entity_id": "alpha"}]}}),
        ),
        (
            "specforge.schema",
            json!({"include_validation_rules": true}),
        ),
        ("specforge.stats", json!({})),
        ("specforge.explain", json!({"code": "w018"})),
        ("specforge.explain", json!({"code": "E047"})),
        ("specforge.inspect", json!({"entity_id": "alpha"})),
        ("specforge.find_definition", json!({"entity_id": "alpha"})),
        ("specforge.find_references", json!({"entity_id": "alpha"})),
        ("specforge.format", json!({"check": true, "diff": true})),
        (
            "specforge.rename",
            json!({"entity_id": "alpha", "new_name": "gamma", "dry_run": true}),
        ),
        (
            "specforge.init",
            json!({"path": new_project.to_str().unwrap()}),
        ),
        (
            "specforge.add_extension",
            json!({"specifier": "@specforge/product", "dry_run": true}),
        ),
        (
            "specforge.add_extension",
            json!({"specifier": "@specforge/software"}),
        ),
        (
            "specforge.remove_extension",
            json!({"name": "@specforge/testing", "dry_run": true}),
        ),
        ("specforge.migrate", json!({"dry_run": true})),
        ("specforge.extensions", json!({})),
        ("specforge.providers", json!({})),
        ("specforge.doctor", json!({"use_cached": true})),
        ("specforge.render", json!({"format": "brief"})),
        ("specforge.infer_progress", json!({})),
        ("specforge.infer_gaps", json!({})),
        ("specforge.infer_session", json!({"action": "start"})),
        (
            "specforge.infer_session",
            json!({"action": "mark_analyzed", "source_file": "src/lib.rs"}),
        ),
        (
            "specforge.find_implementation",
            json!({"entity_id": "alpha"}),
        ),
        (
            "specforge.find_spec_for_source",
            json!({"file_path": "src/lib.rs"}),
        ),
        (
            "specforge.collect",
            json!({"path": collected.path().to_str().unwrap()}),
        ),
        // Tools whose results are arrays or text: no structured result.
        ("specforge.validate", json!({"use_cached": true})),
        ("specforge.search", json!({"query": "alpha"})),
        ("specforge.coverage", json!({})),
        ("specforge.list", json!({})),
        ("specforge.model", json!({})),
        ("specforge.export", json!({})),
    ];
    let mut conforming = std::collections::BTreeSet::new();
    for (name, arguments) in probes {
        let resp = call(
            &mut server,
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        );
        let result = &resp["result"];
        assert_eq!(result["isError"], false, "{name} {arguments}: {resp}");
        let spec = specforge_mcp::tools::core_tool(name).unwrap();
        let Some(structured) = result.get("structuredContent") else {
            continue;
        };
        let schema = spec
            .output_schema()
            .unwrap_or_else(|| panic!("{name} returns an object but declares no outputSchema"));
        let violations = specforge_mcp::json_schema::violations(&schema, structured);
        assert!(
            violations.is_empty(),
            "{name} {arguments}: {violations:#?}\n{structured}"
        );
        conforming.insert(name);
    }
    // Every declared schema was held to a real result.
    for spec in specforge_mcp::tools::CORE_TOOLS {
        if spec.output_schema().is_some() {
            assert!(
                conforming.contains(spec.name),
                "{} never checked",
                spec.name
            );
        }
    }
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a failed call of a tool with an outputSchema carries no structuredContent"
)]
fn a_failed_call_of_a_typed_tool_has_no_structured_content() {
    let (mut server, _dir) = structured_server();
    let resp = call(
        &mut server,
        "tools/call",
        json!({"name": "specforge.inspect", "arguments": {"entity_id": "nope"}}),
    );
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(resp["result"].get("structuredContent").is_none(), "{resp}");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let error: Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        error["code"], "entity_not_found",
        "the McpError is the text"
    );
}

#[specforge_test(
    behavior = "follow_negotiated_mcp_revision",
    verify = "a 2025-03-26 session is listed no outputSchema"
)]
fn output_schemas_are_listed_from_2025_06_18() {
    let listed = |version: &str| {
        let (mut server, _) = negotiated(Some(version));
        let resp = call(&mut server, "tools/list", json!({}));
        resp["result"]["tools"].as_array().unwrap().clone()
    };
    let inspect = |tools: &[Value]| {
        tools
            .iter()
            .find(|t| t["name"] == "specforge.inspect")
            .unwrap()
            .clone()
    };
    assert!(inspect(&listed("2025-06-18"))["outputSchema"].is_object());
    assert!(inspect(&listed("2025-03-26")).get("outputSchema").is_none());
}
