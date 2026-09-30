//! Protocol revision negotiation and what the negotiated revision changes
//! on the wire: JSON-RPC batches (2025-03-26 only), `structuredContent`
//! (2025-06-18 on), and resource templates.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let resp = server
        .handle_message(&request(1, method, params).to_string())
        .unwrap();
    serde_json::from_str(&resp).unwrap()
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
