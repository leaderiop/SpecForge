use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity, SourceSpan};
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{
    EntityId, EntityKind, FieldMap, FieldValue, MethodDecl, Parameter, VerifyStatement,
};
use specforge_test::prelude::*;
use std::path::{Path, PathBuf};

// Leak a per-test temp project: process exits make cleanup unnecessary, and
// a real project root is required now that ops perform real work.
fn attach_project(state: &mut specforge_mcp::state::McpState) {
    let dir = tempfile::TempDir::new().unwrap();
    let config = json!({"name":"t","version":"0.1.0","extensions":[]});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\nfeature beta \"Beta\" {\n    behaviors [alpha]\n}\n",
    )
    .unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    state.project_root = Some(root);
}

fn span_at(file: &str, start_line: usize, start_col: usize, end_line: usize) -> SourceSpan {
    SourceSpan {
        file: file.into(),
        start_line,
        start_col,
        end_line,
        end_col: 0,
    }
}

fn span() -> SourceSpan {
    span_at("test.spec", 1, 0, 5)
}

fn node(id: &str, kind: &str, span: SourceSpan, fields: FieldMap) -> Node {
    Node {
        id: EntityId { raw: id.into() },
        kind: EntityKind { raw: kind.into() },
        title: Some(id.to_uppercase()),
        fields,
        source_span: span,
        methods: Vec::new(),
    }
}

fn edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

fn text_field(key: &str, text: &str) -> FieldMap {
    let mut fields = FieldMap::new();
    fields.push(key.into(), FieldValue::String(text.into()));
    fields
}

/// A kind as an extension registers it; only `testable` matters here.
fn kind_entry(kind: &str, testable: bool) -> specforge_registry::KindRegistryEntry {
    specforge_registry::KindRegistryEntry {
        kind_name: kind.into(),
        description: None,
        source_extension: "@test/ext".into(),
        testable,
        singleton: false,
        supports_verify: testable,
        allowed_verify_kinds: Vec::new(),
        has_body_parser: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: None,
        dot_color: None,
        dot_fillcolor: None,
        open_fields: false,
    }
}

/// An initialized server over a two-entity graph: behavior `alpha`
/// (test.spec:1-5, contract "MUST work", verify unit "works") and feature
/// `beta` (feat.spec:1-3) with a `behaviors` edge beta -> alpha. Behaviors
/// are testable, features are not.
fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    let mut fields = FieldMap::new();
    fields.push("contract".into(), FieldValue::String("MUST work".into()));
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".into(),
            description: "works".into(),
        }]),
    );

    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha".into()),
        fields,
        source_span: span(),
        methods: Vec::new(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta".into()),
        fields: FieldMap::new(),
        source_span: span_at("feat.spec", 1, 0, 3),
        methods: Vec::new(),
    });
    graph.add_edge(edge("beta", "alpha", "behaviors"));
    state.graph = graph;
    state.kind_registry.register(kind_entry("behavior", true));
    state.kind_registry.register(kind_entry("feature", false));
    attach_project(state);

    server
}

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    call(
        server,
        "tools/call",
        json!({"name": name, "arguments": args}),
    )
}

/// A tool call's JSON payload (the text of its first content item).
fn tool_json(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no tool result text in {resp}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("tool text is not JSON ({e}): {text}"))
}

/// Call a tool and parse its JSON payload.
fn tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    tool_json(&call_tool(server, name, args))
}

/// A prompt's structured payload: the assistant message's JSON text.
fn prompt(server: &mut McpServer, name: &str, args: Value) -> Value {
    let resp = call(
        server,
        "prompts/get",
        json!({"name": name, "arguments": args}),
    );
    let text = resp["result"]["messages"][1]["content"]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no prompt payload in {resp}"));
    serde_json::from_str(text).unwrap()
}

/// A resource read's first content item, with its text parsed as JSON.
fn resource(server: &mut McpServer, uri: &str) -> (Value, Value) {
    let resp = call(server, "resources/read", json!({"uri": uri}));
    let content = resp["result"]["contents"][0].clone();
    let text = content["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no resource text in {resp}"));
    let parsed = serde_json::from_str(text).unwrap();
    (content, parsed)
}

/// The params of every recorded event called `name`, oldest first, each
/// without the `timestamp` every event but `mcp_initialized` carries.
fn events(server: &McpServer, name: &str) -> Vec<Value> {
    server
        .state()
        .events
        .iter()
        .filter(|e| e.name == name)
        .map(|e| {
            let mut params = e.params.clone();
            if name != "mcp_initialized" {
                let stamp = params.as_object_mut().unwrap().remove("timestamp");
                assert!(
                    stamp.as_ref().is_some_and(Value::is_string),
                    "{name}: {}",
                    e.params
                );
            }
            params
        })
        .collect()
}

fn assert_tool_invoked(server: &McpServer, tool: &str) {
    let invoked = events(server, "mcp_tool_invoked");
    assert!(
        invoked.iter().any(|p| p["toolName"] == tool),
        "no mcp_tool_invoked for {tool}: {invoked:?}"
    );
}

fn assert_prompt_invoked(server: &McpServer, prompt: &str) {
    let invoked = events(server, "mcp_prompt_invoked");
    assert!(
        invoked.iter().any(|p| p["promptName"] == prompt),
        "no mcp_prompt_invoked for {prompt}: {invoked:?}"
    );
}

fn assert_resource_read(server: &McpServer, uri: &str) {
    let reads = events(server, "mcp_resource_read");
    assert!(
        reads
            .iter()
            .any(|p| p["resourceUri"] == uri && p["format"] == "application/json"),
        "no mcp_resource_read for {uri}: {reads:?}"
    );
}

/// The `id` of each node in a graph payload, sorted.
fn node_ids(payload: &Value) -> Vec<String> {
    let mut ids: Vec<String> = payload["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("no nodes in {payload}"))
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

fn find<'a>(items: &'a Value, key: &str, value: &str) -> &'a Value {
    items
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {items}"))
        .iter()
        .find(|item| item[key] == value)
        .unwrap_or_else(|| panic!("no item with {key} = {value} in {items}"))
}

fn diagnostic(code: &str, message: &str, span: Option<SourceSpan>) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Warning,
        message: message.into(),
        span,
        suggestion: Some(format!("fix {code}")),
    }
}

/// The vendored product extension blob, installable offline.
fn product_wasm() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm")
}

/// A project on disk with `config` as its specforge.json and one spec file.
fn project_dir(config: Value, spec: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(dir.path().join("main.spec"), spec).unwrap();
    dir
}

// NOT LINKED to "MCP Initialize: MCP initialization holds — …": no fixture
// extension contributes MCP surfaces, so surface_contributions_merged cannot
// be exercised through initialize. Everything else in the contract is.
#[test]
fn contract_initialize() {
    let dir = project_dir(json!({"name":"t","version":"0.1.0","extensions":[]}), "");
    let mut server = McpServer::new();

    // No tool call is accepted before initialization completes.
    let early = call_tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(early["error"]["code"], -32600, "{early}");

    let resp = call(
        &mut server,
        "initialize",
        json!({"projectRoot": dir.path().to_str().unwrap()}),
    );
    let result = &resp["result"];
    assert_eq!(result["protocolVersion"], "2025-03-26");
    assert_eq!(result["serverInfo"]["name"], "specforge-mcp");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(result["capabilities"]["resources"]["subscribe"], true);

    // capabilities_returned: every registered tool, resource and prompt.
    let names = |key: &str, field: &str| -> Vec<String> {
        result[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d[field].as_str().unwrap().to_string())
            .collect()
    };
    let tools = names("tools", "name");
    for core in [
        "specforge.query",
        "specforge.export",
        "specforge.trace",
        "specforge.inspect",
        "specforge.format",
        "specforge.doctor",
    ] {
        assert!(
            tools.contains(&core.to_string()),
            "{core} missing: {tools:?}"
        );
    }
    let resources = names("resources", "uri");
    assert!(resources.contains(&"specforge://graph".to_string()));
    assert!(resources.contains(&"specforge://diagnostics".to_string()));
    let prompts = names("prompts", "name");
    assert!(prompts.contains(&"specforge://prompts/context".to_string()));

    // compiler_api_available: the project root was located and used.
    assert_eq!(
        server.state().project_root.as_deref(),
        Some(dir.path()),
        "initialize must adopt the projectRoot it was given"
    );

    // mcp_initialized_emitted, with the advertised counts.
    let initialized = events(&server, "mcp_initialized");
    assert_eq!(
        initialized,
        [json!({
            "tools_registered": tools.len(),
            "resources_registered": resources.len(),
            "prompts_registered": prompts.len(),
            "extensions_loaded": server.state().extension_info.len(),
            "surface_tools_registered": 0,
            "surface_resources_registered": 0,
            "auto_promoted_tools": 0,
        })]
    );

    // Tool calls are accepted once initialized.
    let after = call_tool(&mut server, "specforge.stats", json!({}));
    assert!(after["error"].is_null(), "{after}");
}

#[specforge_test(
    behavior = "mcp_shutdown",
    verify = "MCP Shutdown: MCP shutdown holds — server_initialized, notifications_flushed, subscriptions_removed, wasm_engines_released, shutdown_emitted"
)]
fn contract_shutdown() {
    let mut server = test_server();
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://graph"}),
    );
    call(
        &mut server,
        "resources/subscribe",
        json!({"uri": "specforge://diagnostics"}),
    );
    // A compile left a graph notification pending for the subscriber.
    specforge_mcp::notifications::enqueue_compile_notifications(
        server.state_mut(),
        &Graph::new(),
        &[],
    );
    assert_eq!(server.state().notification_outbox.len(), 1);

    let resp = call(&mut server, "shutdown", json!({}));
    assert_eq!(resp["result"], json!({}), "{resp}");

    // notifications_flushed: the pending notification still reaches the
    // client after the shutdown response.
    let delivered = server.take_notifications();
    assert_eq!(delivered.len(), 1, "{delivered:?}");
    let mut added: Vec<&str> = delivered[0]["params"]["added_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    added.sort();
    assert_eq!(added, ["alpha", "beta"]);

    // subscriptions_removed: none left, and each removal was announced.
    assert!(server.state().subscriptions.is_empty());
    let mut removed = events(&server, "mcp_subscription_removed");
    removed.sort_by_key(|p| p["subscriptionType"].to_string());
    assert_eq!(
        removed,
        [
            json!({"subscriptionType": "specforge/diagnosticsChanged", "clientId": "default"}),
            json!({"subscriptionType": "specforge/graphChanged", "clientId": "default"}),
        ]
    );

    // wasm_engines_released: nothing compiled survives shutdown.
    let state = server.state();
    assert_eq!(state.graph.node_count(), 0);
    assert!(state.manifests.is_empty());
    assert!(state.surface_entries.is_empty());
    assert!(state.project_root.is_none());

    // shutdown_emitted, with what it released.
    let shutdown = events(&server, "mcp_server_shutdown");
    assert_eq!(shutdown.len(), 1, "{shutdown:?}");
    assert_eq!(shutdown[0]["pending_notifications_flushed"], 1);
    assert_eq!(shutdown[0]["subscriptions_released"], 2);
    assert_eq!(shutdown[0]["wasm_engines_released"], 0);

    // No new tool calls during teardown.
    let late = call_tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(late["error"]["code"], -32600, "{late}");
}

/// The I020 report for an unknown kind in a `kinds` filter.
fn unknown_kind(kind: &str, suggestion: Option<&str>) -> Value {
    json!({
        "code": "I020",
        "severity": "Info",
        "message": format!("unknown entity kind '{kind}'"),
        "span": null,
        "suggestion": suggestion.map(|s| format!("did you mean '{s}'?")),
    })
}

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "Provide MCP Query Tool: MCP query tool holds — graph_available, subgraph_returned, unknown_kinds_reported, tool_invoked_emitted"
)]
fn contract_query() {
    let mut server = test_server();
    // graph_available: the tool reads the server's compiled graph.
    assert_eq!(server.state().graph.node_count(), 2);
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    let parsed = tool_json(&resp);
    // subgraph_returned: alpha and its neighbor beta, with the edge.
    assert_eq!(node_ids(&parsed), ["alpha", "beta"]);
    assert_eq!(
        parsed["edges"],
        json!([{"source": "beta", "target": "alpha", "label": "behaviors"}])
    );
    assert_eq!(
        find(&parsed["nodes"], "id", "alpha")["fields"]["contract"],
        "MUST work"
    );
    // Only known kinds asked for: nothing to report.
    assert!(resp["result"]["_meta"].is_null(), "{resp}");

    // unknown_kinds_reported: the kind filter keeps only matching nodes,
    // unknown kinds are dropped, and each one is reported with I020 in the
    // response metadata, with a suggestion when a known kind is close.
    let filtered_args =
        json!({"entity_id": "alpha", "kinds": ["behavior", "behaviour", "nonexistent"]});
    let filtered = call_tool(&mut server, "specforge.query", filtered_args.clone());
    assert!(filtered["result"]["isError"].is_null(), "{filtered}");
    assert_eq!(node_ids(&tool_json(&filtered)), ["alpha"]);
    assert_eq!(
        filtered["result"]["_meta"]["diagnostics"],
        json!([
            unknown_kind("behaviour", Some("behavior")),
            unknown_kind("nonexistent", None),
        ])
    );

    // tool_invoked_emitted: one event per call, naming the entity.
    assert_eq!(
        events(&server, "mcp_tool_invoked"),
        [
            json!({
                "toolName": "specforge.query",
                "category": "core",
                "params": json!({"entity_id": "alpha"}).to_string(),
                "entityId": "alpha",
            }),
            json!({
                "toolName": "specforge.query",
                "category": "core",
                "params": filtered_args.to_string(),
                "entityId": "alpha",
            }),
        ]
    );
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "Provide MCP Export Tool: MCP export tool holds — graph_available, format_produced, token_budget_enforced, tool_invoked_emitted"
)]
fn contract_export() {
    let mut server = test_server();

    // format_produced: each format carries the graph in its own shape.
    let graph = tool(&mut server, "specforge.export", json!({"format": "graph"}));
    assert_eq!(node_ids(&graph), ["alpha", "beta"]);
    assert_eq!(
        find(&graph["nodes"], "id", "alpha")["fields"]["contract"],
        "MUST work"
    );
    assert_eq!(find(&graph["nodes"], "id", "alpha")["file"], "test.spec");
    assert_eq!(
        graph["edges"],
        json!([{"source": "beta", "target": "alpha", "label": "behaviors"}])
    );

    let context = tool(
        &mut server,
        "specforge.export",
        json!({"format": "context"}),
    );
    let alpha = find(&context["nodes"], "id", "alpha");
    assert_eq!(alpha["contract"], "MUST work");
    assert_eq!(
        alpha["verify"],
        json!([{"kind": "unit", "description": "works"}])
    );
    assert!(
        alpha.get("fields").is_none(),
        "context flattens fields: {alpha}"
    );

    let brief = tool(&mut server, "specforge.export", json!({"format": "brief"}));
    assert_eq!(node_ids(&brief), ["alpha", "beta"]);
    let alpha = find(&brief["nodes"], "id", "alpha");
    assert_eq!(alpha["title"], "Alpha");
    assert!(alpha.get("contract").is_none() && alpha.get("fields").is_none());
    assert_eq!(brief["edges"].as_array().unwrap().len(), 1);

    // token_budget_enforced: an orphan with a long contract is the first
    // thing a tight budget drops; the connected pair stays.
    let long = "MUST ".repeat(200);
    server.state_mut().graph.add_node(node(
        "gamma",
        "behavior",
        span_at("test.spec", 10, 0, 12),
        text_field("contract", &long),
    ));
    let budgeted = tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "max_tokens": 200}),
    );
    assert_eq!(node_ids(&budgeted), ["alpha", "beta"], "{budgeted}");
    assert_eq!(budgeted["token_budget"]["budget_tokens"], 200);
    assert_eq!(
        budgeted["token_budget"]["truncated_entities"],
        json!(["gamma"])
    );

    assert_tool_invoked(&server, "specforge.export");
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "max_tokens truncates output to fit token budget"
)]
fn contract_export_max_tokens() {
    let mut server = test_server();

    // Descriptor advertises max_tokens
    let resp = call(&mut server, "tools/list", json!({}));
    let export_tool = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "specforge.export")
        .expect("specforge.export descriptor must be registered");
    assert!(
        export_tool["inputSchema"]["properties"]["max_tokens"].is_object(),
        "export descriptor must declare max_tokens"
    );

    // Tool truncates graph output when max_tokens is set
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "max_tokens": 5}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(
        parsed["token_budget"].is_object(),
        "graph export with max_tokens must include token_budget metadata"
    );
    assert!(
        parsed["nodes"].as_array().unwrap().len() < 2,
        "tiny budget must truncate the 2-node graph"
    );
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "Provide MCP Trace Tool: MCP trace tool holds — graph_available, trace_result_returned, gaps_identified, tool_invoked_emitted"
)]
fn contract_trace() {
    let mut server = test_server();

    // trace_result_returned: a TraceChain for an entity.
    let chain = tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(chain["entity_id"], "alpha");
    assert_eq!(chain["entity_kind"], "behavior");
    assert_eq!(
        chain["upstream"],
        json!([{"entity_id": "beta", "entity_kind": "feature", "edge_label": "behaviors", "depth": 1}])
    );
    assert_eq!(chain["downstream"], json!([]));
    // gaps_identified: alpha has nothing downstream, beta nothing upstream.
    assert_eq!(chain["gaps"], json!(["no downstream links"]));
    let beta = tool(&mut server, "specforge.trace", json!({"entity_id": "beta"}));
    assert_eq!(beta["gaps"], json!(["no upstream links"]));

    // ...or a McpTracePlanResult for a plan, flagging what the graph lacks.
    let plan = tool(
        &mut server,
        "specforge.trace",
        json!({"plan": {"entries": [{"entity_id": "alpha"}, {"entity_id": "ghost"}]}}),
    );
    assert_eq!(plan["affected_entities"], json!(["alpha"]));
    let gaps = plan["gaps"].as_array().unwrap();
    assert!(
        gaps.iter()
            .any(|g| g["target_entity"] == "ghost" && g["missing_link_type"] == "unresolved_entity"),
        "the unresolved plan entry is a gap: {gaps:?}"
    );

    assert_tool_invoked(&server, "specforge.trace");
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "Provide MCP Search Tool: MCP search tool holds — graph_available, filtered_results_returned, unknown_kinds_reported, tool_invoked_emitted"
)]
fn contract_search() {
    let mut server = test_server();
    let ids = |resp: &Value| -> Vec<String> {
        tool_json(resp)
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["entity_id"].as_str().unwrap().to_string())
            .collect()
    };

    // graph_available: results come from the server's compiled graph.
    let by_text = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    assert_eq!(ids(&by_text), ["alpha"]);
    let hit = &tool_json(&by_text)[0];
    assert_eq!(hit["kind"], "behavior");
    assert_eq!(hit["title"], "Alpha");
    assert_eq!(hit["file_path"], "test.spec");
    assert_eq!(hit["line"], 1);
    assert!(by_text["result"]["_meta"].is_null(), "{by_text}");

    // filtered_results_returned: filters combine with AND — kind feature
    // and an empty query is beta; kind feature and text alpha is nothing.
    let by_kind = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "", "kinds": ["feature"]}),
    );
    assert_eq!(ids(&by_kind), ["beta"]);
    let both = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "kinds": ["feature"]}),
    );
    assert_eq!(ids(&both), Vec::<String>::new());

    // unknown_kinds_reported: unknown kinds match nothing, are not an
    // error, and each one is reported with I020 in the response metadata.
    let unknown = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "", "kinds": ["featur", "nonexistent"]}),
    );
    assert!(unknown["result"]["isError"].is_null(), "{unknown}");
    assert_eq!(ids(&unknown), Vec::<String>::new());
    assert_eq!(
        unknown["result"]["_meta"]["diagnostics"],
        json!([
            unknown_kind("featur", Some("feature")),
            unknown_kind("nonexistent", None),
        ])
    );
    // A known kind beside an unknown one still filters.
    let mixed = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "", "kinds": ["behavior", "behaviour"]}),
    );
    assert_eq!(ids(&mixed), ["alpha"]);
    assert_eq!(
        mixed["result"]["_meta"]["diagnostics"],
        json!([unknown_kind("behaviour", Some("behavior"))])
    );

    // tool_invoked_emitted: one event per call, carrying its arguments.
    let invoked = events(&server, "mcp_tool_invoked");
    let params: Vec<&str> = invoked
        .iter()
        .map(|p| {
            assert_eq!(p["toolName"], "specforge.search", "{p}");
            assert_eq!(p["category"], "core", "{p}");
            p["params"].as_str().unwrap()
        })
        .collect();
    assert_eq!(
        params,
        [
            json!({"query": "alpha"}).to_string(),
            json!({"query": "", "kinds": ["feature"]}).to_string(),
            json!({"query": "alpha", "kinds": ["feature"]}).to_string(),
            json!({"query": "", "kinds": ["featur", "nonexistent"]}).to_string(),
            json!({"query": "", "kinds": ["behavior", "behaviour"]}).to_string(),
        ]
    );
}

#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "Provide MCP Stats Tool: MCP stats tool holds — graph_available, stats_returned, latest_state_reflected, tool_invoked_emitted"
)]
fn contract_stats() {
    let mut server = test_server();
    let stats = tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(
        stats["entity_counts"],
        json!([{"kind": "behavior", "count": 1}, {"kind": "feature", "count": 1}])
    );
    assert_eq!(stats["edge_count"], 1);
    assert_eq!(stats["orphan_count"], 0);
    assert!(stats["coverage_pct"].is_number(), "{stats}");
    assert_eq!(
        stats["diagnostic_summary"],
        json!({"errors": 0, "warnings": 0, "infos": 0})
    );

    // latest_state_reflected: a new orphan and a warning show up at once.
    let state = server.state_mut();
    state.graph.add_node(node(
        "gamma",
        "behavior",
        span_at("test.spec", 10, 0, 12),
        FieldMap::new(),
    ));
    state
        .diagnostics
        .push(diagnostic("W001", "a warning", None));
    let stats = tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(
        stats["entity_counts"],
        json!([{"kind": "behavior", "count": 2}, {"kind": "feature", "count": 1}])
    );
    assert_eq!(stats["orphan_count"], 1);
    assert_eq!(stats["diagnostic_summary"]["warnings"], 1);

    assert_tool_invoked(&server, "specforge.stats");
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "Provide MCP Inspect Tool: MCP inspect tool holds — graph_available, entity_details_returned, tool_invoked_emitted"
)]
fn contract_inspect() {
    let mut server = test_server();
    let diagnostics = &mut server.state_mut().diagnostics;
    // One diagnostic inside alpha's span, one in beta's file.
    diagnostics.push(diagnostic(
        "W001",
        "inside alpha",
        Some(span_at("test.spec", 2, 4, 2)),
    ));
    diagnostics.push(diagnostic(
        "W002",
        "inside beta",
        Some(span_at("feat.spec", 2, 0, 2)),
    ));

    let details = tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(details["entity_id"], "alpha");
    assert_eq!(details["kind"], "behavior");
    assert_eq!(details["title"], "Alpha");
    assert_eq!(details["contract"], "MUST work");
    assert_eq!(details["fields"]["contract"], "MUST work");
    assert_eq!(details["verify_declarations"], json!(["unit works"]));
    assert_eq!(details["references"], json!(["beta"]));
    assert_eq!(details["coverage_status"], "uncovered");
    let codes: Vec<&str> = details["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["W001"]);
    assert_eq!(details["source_span"]["file"], "test.spec");
    assert_eq!(details["source_span"]["start_line"], 1);

    assert_tool_invoked(&server, "specforge.inspect");
}

#[specforge_test(
    behavior = "provide_mcp_find_definition_tool",
    verify = "Provide MCP Find Definition Tool: MCP find definition tool holds — graph_available, source_location_returned, tool_invoked_emitted"
)]
fn contract_find_definition() {
    let mut server = test_server();
    server.state_mut().graph.add_node(node(
        "gamma",
        "behavior",
        span_at("more/gamma.spec", 7, 2, 9),
        FieldMap::new(),
    ));

    let alpha = tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(
        alpha,
        json!({"entity_id": "alpha", "file_path": "test.spec", "line": 1, "column": 0})
    );
    let gamma = tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "gamma"}),
    );
    assert_eq!(
        gamma,
        json!({"entity_id": "gamma", "file_path": "more/gamma.spec", "line": 7, "column": 2})
    );

    assert_tool_invoked(&server, "specforge.find_definition");
}

#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "Provide MCP Find References Tool: MCP find references tool holds — graph_available, references_returned, empty_list_for_unreferenced, tool_invoked_emitted"
)]
fn contract_find_references() {
    let mut server = test_server();

    // references_returned: beta references alpha, with where beta lives.
    let refs = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "alpha"}),
    );
    let refs = tool_json(&refs);
    assert_eq!(refs["entity_id"], "alpha");
    let locations = refs["locations"].as_array().unwrap();
    assert_eq!(locations.len(), 1, "{locations:?}");
    assert_eq!(locations[0]["referencing_entity_id"], "beta");
    assert_eq!(locations[0]["source_span"]["file"], "feat.spec");
    assert_eq!(locations[0]["source_span"]["start_line"], 1);
    assert_eq!(locations[0]["source_span"]["start_col"], 0);

    // empty_list_for_unreferenced: nothing references beta; not an error.
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "beta"}),
    );
    assert!(resp["error"].is_null(), "{resp}");
    assert!(resp["result"]["isError"].as_bool() != Some(true), "{resp}");
    assert_eq!(tool_json(&resp)["locations"], json!([]));

    assert_tool_invoked(&server, "specforge.find_references");
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "Provide MCP Outline Tool: MCP outline tool holds — graph_available, outline_returned, tool_invoked_emitted"
)]
fn contract_outline() {
    let mut server = test_server();
    let mut gamma = node(
        "gamma",
        "port",
        span_at("test.spec", 7, 0, 12),
        FieldMap::new(),
    );
    gamma.methods.push(MethodDecl {
        name: "check".into(),
        params: vec![Parameter {
            name: "x".into(),
            ty: "Int".into(),
            optional: false,
            annotations: Vec::new(),
        }],
        returns: Some("Bool".into()),
        span: span_at("test.spec", 8, 4, 8),
    });
    server.state_mut().graph.add_node(gamma);

    let outline = tool(
        &mut server,
        "specforge.outline",
        json!({"file": "test.spec"}),
    );
    // Every entity in test.spec (beta lives in feat.spec), by line.
    let entries = outline.as_array().unwrap();
    let ids: Vec<&str> = entries
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["alpha", "gamma"]);
    assert_eq!(entries[0]["kind"], "behavior");
    assert_eq!(entries[0]["title"], "Alpha");
    assert_eq!(entries[0]["range"]["start_line"], 1);
    assert_eq!(entries[0]["range"]["end_line"], 5);
    assert_eq!(entries[1]["kind"], "port");
    assert_eq!(entries[1]["range"]["start_line"], 7);
    assert_eq!(entries[1]["range"]["end_line"], 12);
    // Nested children: gamma's method member.
    let children = entries[1]["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["entity_id"], "gamma.check");
    assert_eq!(children[0]["kind"], "method");
    assert_eq!(children[0]["title"], "check(x: Int) -> Bool");
    assert_eq!(children[0]["range"]["start_line"], 8);

    assert_tool_invoked(&server, "specforge.outline");
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "Provide MCP Coverage Tool: MCP coverage tool holds — graph_available, coverage_returned, testability_respected, tool_invoked_emitted"
)]
fn contract_coverage() {
    let mut server = test_server();

    // coverage_returned: behaviors are testable, features are not.
    let coverage = tool(&mut server, "specforge.coverage", json!({}));
    assert_eq!(
        coverage,
        json!([{
            "entity_id": "alpha",
            "kind": "behavior",
            "status": "uncovered",
            "declared": true,
            "linked": false,
            "evidence_collected": false,
            "obligations": 1,
            "proven": 0,
            "unproven": ["works"],
        }])
    );

    // Recorded evidence: a passing test that names the obligation.
    let root = server.state().project_root.clone().unwrap();
    std::fs::write(
        root.join("specforge-report.json"),
        json!({"results": {"alpha": {"tests": [
            {"name": "alpha_works", "status": "pass", "verify": "works"}
        ]}}})
        .to_string(),
    )
    .unwrap();
    let coverage = tool(&mut server, "specforge.coverage", json!({}));
    let alpha = find(&coverage, "entity_id", "alpha");
    assert_eq!(alpha["status"], "covered");
    assert_eq!(alpha["proven"], 1);
    assert_eq!(alpha["linked"], true);
    assert_eq!(alpha["unproven"], json!([]));

    // testability_respected: the registry, not the kind name, decides.
    server
        .state_mut()
        .kind_registry
        .register(kind_entry("feature", true));
    let coverage = tool(&mut server, "specforge.coverage", json!({}));
    let beta = find(&coverage, "entity_id", "beta");
    assert_eq!(beta["obligations"], 0);
    assert_eq!(beta["status"], "uncovered");

    assert_tool_invoked(&server, "specforge.coverage");
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "Provide MCP Schema Tool: MCP schema tool holds — graph_available, schema_returned, tool_invoked_emitted"
)]
fn contract_schema() {
    let mut server = test_server();
    let schema = tool(&mut server, "specforge.schema", json!({}));
    assert_eq!(
        schema["entity_kinds"],
        json!({"behavior": ["contract", "verify"], "feature": []})
    );
    assert_eq!(schema["edge_labels"], json!(["behaviors"]));
    assert!(schema["schema_version"].is_string(), "{schema}");

    // Optionally filtered to one kind.
    let feature = tool(&mut server, "specforge.schema", json!({"kind": "feature"}));
    assert_eq!(feature["entity_kinds"], json!({"feature": []}));

    assert_tool_invoked(&server, "specforge.schema");
}

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "Provide MCP Context Prompt: MCP context prompt holds — graph_available, context_returned, hints_included, prompt_invoked_emitted"
)]
fn contract_context_prompt() {
    let mut server = test_server();
    // An invariant nothing connects to alpha.
    server.state_mut().graph.add_node(node(
        "gamma",
        "invariant",
        span_at("inv.spec", 1, 0, 3),
        text_field("guarantee", "never negative"),
    ));

    let context = prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha", "structural_constraints": ["gamma"]}),
    );
    // context_returned: contract, related entities, verify declarations.
    assert_eq!(context["entity_id"], "alpha");
    assert_eq!(context["kind"], "behavior");
    assert_eq!(context["contract_text"], "MUST work");
    assert_eq!(context["upstream_entities"], json!(["beta"]));
    assert_eq!(context["downstream_entities"], json!([]));
    assert_eq!(context["verify_expectations"], json!(["unit works"]));
    // hints_included: gamma rides along although no edge reaches it.
    assert_eq!(context["structural_constraints"], json!(["gamma"]));
    let hint = &context["structural_constraint_entities"][0];
    assert_eq!(hint["entity_id"], "gamma");
    assert_eq!(hint["kind"], "invariant");
    assert_eq!(hint["fields"]["guarantee"], "never negative");

    // As an MCP prompt argument string, the list is comma-separated.
    let context = prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha", "structural_constraints": "gamma, beta"}),
    );
    assert_eq!(context["structural_constraints"], json!(["gamma", "beta"]));

    assert_prompt_invoked(&server, "specforge://prompts/context");
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "Provide MCP Review Prompt: MCP review prompt holds — graph_available, coverage_analysis_returned, gaps_identified, prompt_invoked_emitted"
)]
fn contract_review_prompt() {
    let mut server = test_server();
    // gamma: a testable behavior of beta with no verify declarations;
    // delta: two hops from beta, outside depth 1.
    let state = server.state_mut();
    state.graph.add_node(node(
        "gamma",
        "behavior",
        span_at("test.spec", 7, 0, 9),
        FieldMap::new(),
    ));
    state.graph.add_node(node(
        "delta",
        "behavior",
        span_at("test.spec", 11, 0, 13),
        FieldMap::new(),
    ));
    state.graph.add_edge(edge("beta", "gamma", "behaviors"));
    state.graph.add_edge(edge("gamma", "delta", "depends_on"));

    let review = prompt(
        &mut server,
        "specforge://prompts/review",
        json!({"entity_id": "beta", "depth": 1}),
    );
    assert_eq!(review["entity_id"], "beta");

    // coverage_analysis_returned: the testable entities within one hop.
    let summary = review["coverage_summary"].as_array().unwrap();
    let covered: Vec<&str> = summary
        .iter()
        .map(|c| c["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(covered, ["alpha", "gamma"]);
    let alpha = find(&review["coverage_summary"], "entity_id", "alpha");
    assert_eq!(alpha["status"], "uncovered");
    // gaps_identified: the uncovered verify text and the missing evidence.
    assert_eq!(alpha["unproven"], json!(["works"]));
    assert_eq!(alpha["linked"], false);
    assert_eq!(alpha["evidence_collected"], false);
    // ...and missing verification coverage.
    let findings = review["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f["entity_id"] == "gamma"
            && f["severity"] == "warning"
            && f["message"]
                .as_str()
                .unwrap()
                .contains("no verify declarations")),
        "{findings:?}"
    );
    assert!(
        !findings.iter().any(|f| f["entity_id"] == "alpha"),
        "alpha declares verify: {findings:?}"
    );

    assert_prompt_invoked(&server, "specforge://prompts/review");
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "Provide MCP Trace Prompt: MCP trace prompt holds — graph_available, gaps_returned, affected_entities_listed, prompt_invoked_emitted"
)]
fn contract_trace_prompt() {
    let mut server = test_server();

    // affected_entities_listed: the plan's entry and what its chain reaches.
    let trace = prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"plan": {"entries": [{"entity_id": "alpha"}, {"entity_id": "ghost"}]}}),
    );
    assert_eq!(trace["affected_entities"], json!(["alpha", "beta"]));
    assert_eq!(trace["unverified_entities"], json!(["beta"]));

    // gaps_returned: the entry the graph lacks, with its gap context.
    let gaps = trace["coverage_gaps"].as_array().unwrap();
    let ghost = gaps
        .iter()
        .find(|g| g["target_entity"] == "ghost")
        .unwrap_or_else(|| panic!("no gap for the unresolved entry: {gaps:?}"));
    assert!(
        ghost["gap_context"].as_str().is_some_and(|c| !c.is_empty()),
        "{ghost}"
    );
    // Deterministic: the same plan yields the same gaps.
    let again = prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"plan": {"entries": [{"entity_id": "alpha"}, {"entity_id": "ghost"}]}}),
    );
    assert_eq!(again, trace);

    assert_prompt_invoked(&server, "specforge://prompts/trace");
}

#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "Provide MCP Explore Prompt: MCP explore prompt holds — graph_available, exploration_returned, bfs_from_entity, prompt_invoked_emitted"
)]
fn contract_explore_prompt() {
    let mut server = test_server();
    // alpha <-behaviors- beta -invariants-> gamma; delta is an orphan.
    let state = server.state_mut();
    state.graph.add_node(node(
        "gamma",
        "invariant",
        span_at("inv.spec", 1, 0, 3),
        FieldMap::new(),
    ));
    state.graph.add_node(node(
        "delta",
        "behavior",
        span_at("test.spec", 11, 0, 13),
        FieldMap::new(),
    ));
    state.graph.add_edge(edge("beta", "gamma", "invariants"));

    // exploration_returned: starting points, hubs and orphans.
    let explore = prompt(&mut server, "specforge://prompts/explore", json!({}));
    assert_eq!(explore["high_connectivity"][0], "beta");
    assert_eq!(explore["orphan_nodes"], json!(["delta"]));
    assert_eq!(explore["starting_points"][0], "beta");
    assert_eq!(explore["relationship_paths"], json!([]));

    // bfs_from_entity: paths from alpha, nearest first.
    let from_alpha = prompt(
        &mut server,
        "specforge://prompts/explore",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(
        from_alpha["relationship_paths"],
        json!([
            {"from_entity": "alpha", "to_entity": "beta", "edge_types": ["behaviors"], "path_length": 1},
            {"from_entity": "alpha", "to_entity": "gamma", "edge_types": ["behaviors", "invariants"], "path_length": 2},
        ])
    );
    // A kind filter keeps the paths that end at that kind.
    let invariants = prompt(
        &mut server,
        "specforge://prompts/explore",
        json!({"entity_id": "alpha", "kind": "invariant"}),
    );
    let ends: Vec<&str> = invariants["relationship_paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["to_entity"].as_str().unwrap())
        .collect();
    assert_eq!(ends, ["gamma"]);

    assert_prompt_invoked(&server, "specforge://prompts/explore");
}

/// Register an extension tool and resource, each with its surface entry.
fn add_extension_surface(server: &mut McpServer, name: &str, enabled: bool) {
    use specforge_mcp::types::{McpResourceDescriptor, McpToolDescriptor};
    use specforge_registry::{SurfaceRegistryEntry, SurfaceType};
    let state = server.state_mut();
    state.tool_registry.push(McpToolDescriptor {
        name: format!("ext.{name}"),
        description: format!("{name} tool"),
        input_schema: json!({"type": "object"}),
        category: Some("extension".into()),
    });
    state.resource_registry.push(McpResourceDescriptor {
        uri: format!("specforge://ext/{name}"),
        name: format!("ext-{name}"),
        description: None,
        mime_type: Some("application/json".into()),
    });
    for (surface_type, contribution) in [
        (SurfaceType::McpTool, format!("ext.{name}")),
        (SurfaceType::McpResource, format!("ext-{name}")),
    ] {
        state.surface_entries.push(SurfaceRegistryEntry {
            surface_type,
            contribution_name: contribution,
            extension_name: "@test/ext".into(),
            export_name: format!("export_{name}"),
            enabled,
        });
    }
}

// NOT LINKED to "List MCP Tools: listing MCP tools holds — …": CLI commands
// are never auto-promoted into the MCP tool list
// (auto_promote_commands_to_mcp_tools is not wired into the server), so
// complete_list_returned does not hold for them.
#[test]
fn contract_list_tools() {
    let mut server = test_server();
    add_extension_surface(&mut server, "on", true);
    add_extension_surface(&mut server, "off", false);

    let resp = call(&mut server, "tools/list", json!({}));
    let names: Vec<&str> = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for core in [
        "specforge.query",
        "specforge.validate",
        "specforge.export",
        "specforge.trace",
        "specforge.search",
        "specforge.schema",
        "specforge.coverage",
        "specforge.stats",
        "specforge.analyze",
        "specforge.inspect",
        "specforge.find_definition",
        "specforge.find_references",
        "specforge.outline",
        "specforge.suggest_fixes",
        "specforge.format",
        "specforge.rename",
        "specforge.init",
        "specforge.add_extension",
        "specforge.remove_extension",
        "specforge.migrate",
        "specforge.extensions",
        "specforge.providers",
        "specforge.doctor",
        "specforge.collect",
        "specforge.render",
        "specforge.list",
    ] {
        assert!(names.contains(&core), "{core} missing: {names:?}");
    }
    // Extension-contributed tools are listed; disabled ones are not.
    assert!(names.contains(&"ext.on"), "{names:?}");
    assert!(!names.contains(&"ext.off"), "{names:?}");

    // The count is what the client got: disabled surfaces excluded.
    assert_eq!(
        events(&server, "mcp_discovery_invoked"),
        [json!({"discoveryType": "tools", "resultCount": names.len()})]
    );
}

#[specforge_test(
    behavior = "list_mcp_resources",
    verify = "List MCP Resources: listing MCP resources holds — server_initialized, complete_list_returned, disabled_excluded, discovery_emitted"
)]
fn contract_list_resources() {
    let mut server = test_server();
    add_extension_surface(&mut server, "on", true);
    add_extension_surface(&mut server, "off", false);

    let resp = call(&mut server, "resources/list", json!({}));
    let mut uris: Vec<&str> = resp["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    uris.sort();
    // complete_list_returned: every core resource plus the enabled
    // extension's; disabled_excluded: not the disabled one.
    assert_eq!(
        uris,
        [
            "specforge://brief",
            "specforge://context",
            "specforge://context/{entity_id}",
            "specforge://diagnostics",
            "specforge://entities/{kind}",
            "specforge://ext/on",
            "specforge://graph",
            "specforge://graph/{entity_id}",
            "specforge://schema",
        ]
    );

    // The count is what the client got: disabled surfaces excluded.
    assert_eq!(
        events(&server, "mcp_discovery_invoked"),
        [json!({"discoveryType": "resources", "resultCount": uris.len()})]
    );

    // server_initialized: an uninitialized server lists nothing.
    let mut fresh = McpServer::new();
    let resp = call(&mut fresh, "resources/list", json!({}));
    assert_eq!(resp["error"]["code"], -32600, "{resp}");
}

#[specforge_test(
    behavior = "list_mcp_prompts",
    verify = "List MCP Prompts: listing MCP prompts holds — server_initialized, complete_list_returned, discovery_emitted"
)]
fn contract_list_prompts() {
    let mut server = test_server();
    // An extension-contributed prompt, registered beside the core ones.
    server
        .state_mut()
        .prompt_registry
        .push(specforge_mcp::types::McpPromptDescriptor {
            name: "specforge://prompts/ext_review".into(),
            description: "Extension review".into(),
            arguments: None,
        });

    let resp = call(&mut server, "prompts/list", json!({}));
    let mut names: Vec<&str> = resp["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "specforge://prompts/context",
            "specforge://prompts/explore",
            "specforge://prompts/ext_review",
            "specforge://prompts/infer",
            "specforge://prompts/review",
            "specforge://prompts/trace",
        ]
    );

    // The count is what the client got: disabled surfaces excluded.
    assert_eq!(
        events(&server, "mcp_discovery_invoked"),
        [json!({"discoveryType": "prompts", "resultCount": names.len()})]
    );

    let mut fresh = McpServer::new();
    let resp = call(&mut fresh, "prompts/list", json!({}));
    assert_eq!(resp["error"]["code"], -32600, "{resp}");
}

#[specforge_test(
    behavior = "guard_mcp_reinitialization",
    verify = "Guard MCP Reinitialization: MCP reinitialization guard holds — server_initialized, reinit_rejected, session_unaffected, error_handled_emitted"
)]
fn contract_guard_reinit() {
    let mut server = test_server();
    let root = server.state().project_root.clone();
    let tools = server.state().tool_registry.len();
    let other = tempfile::TempDir::new().unwrap();

    // reinit_rejected: -32600, even when naming another project.
    let resp = call(
        &mut server,
        "initialize",
        json!({"projectRoot": other.path().to_str().unwrap()}),
    );
    assert_eq!(resp["error"]["code"], -32600, "{resp}");

    // session_unaffected: same project, graph and registries.
    let state = server.state();
    assert_eq!(state.project_root, root);
    assert_eq!(state.tool_registry.len(), tools);
    assert_eq!(state.graph.node_count(), 2);
    let stats = tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(stats["edge_count"], 1);

    // error_handled_emitted.
    let handled = events(&server, "mcp_protocol_error_handled");
    assert!(
        handled
            .iter()
            .any(|p| p["method"] == "initialize" && p["errorCode"] == -32600),
        "{handled:?}"
    );
}

#[test]
fn contract_cancel() {
    let mut server = test_server();
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 1}));
    assert!(resp["result"].is_object() || resp["result"].is_null());
}

#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "Handle MCP Protocol Error: MCP protocol error handling holds — mcp_protocol_available, standard_error_returned, no_state_leaked, server_operational, error_handled_emitted"
)]
fn contract_protocol_error() {
    let mut server = test_server();
    let root = server
        .state()
        .project_root
        .clone()
        .unwrap()
        .display()
        .to_string();

    // standard_error_returned: the JSON-RPC code, with a readable message.
    let unknown = call(&mut server, "nonexistent/method", json!({}));
    assert_eq!(unknown["jsonrpc"], "2.0");
    assert_eq!(unknown["id"], 1);
    assert_eq!(unknown["error"]["code"], -32601);
    assert_eq!(
        unknown["error"]["message"],
        "Method not found: nonexistent/method"
    );

    let parse = server.handle_message("{not json").unwrap();
    let parse: Value = serde_json::from_str(&parse).unwrap();
    assert_eq!(parse["error"]["code"], -32700);
    assert_eq!(parse["error"]["message"], "Parse error");

    let missing = call(&mut server, "tools/call", json!({}));
    assert_eq!(missing["error"]["code"], -32602);

    // no_state_leaked: no paths, panics or addresses in the messages.
    for resp in [&unknown, &parse, &missing] {
        let message = resp["error"]["message"].as_str().unwrap();
        assert!(!message.is_empty());
        for leak in [root.as_str(), "panicked", "0x", ".rs:", "src/"] {
            assert!(!message.contains(leak), "{message:?} leaks {leak:?}");
        }
    }

    // server_operational: requests after the errors still succeed.
    let listed = call(&mut server, "tools/list", json!({}));
    assert!(!listed["result"]["tools"].as_array().unwrap().is_empty());
    assert_eq!(server.state().graph.node_count(), 2);

    // error_handled_emitted, once per error with its code.
    let codes: Vec<i64> = events(&server, "mcp_protocol_error_handled")
        .iter()
        .map(|p| p["errorCode"].as_i64().unwrap())
        .collect();
    assert_eq!(codes, [-32601, -32700, -32602]);
}

#[test]
fn contract_validate() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    assert!(
        resp["error"].is_object() || resp["result"]["content"][0]["text"].is_string(),
        "validate must return error or diagnostics text"
    );
}

#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "Provide MCP Suggest Fixes Tool: MCP suggest fixes tool holds — graph_available, fixes_returned, empty_for_clean, tool_invoked_emitted"
)]
fn contract_suggest_fixes() {
    let mut server = test_server();
    server.state_mut().diagnostics.push(diagnostic(
        "W001",
        "inside alpha",
        Some(span_at("test.spec", 2, 4, 2)),
    ));

    // fixes_returned: the diagnostic's fix, with title, edits, diagnostic.
    let fixes = tool(&mut server, "specforge.suggest_fixes", json!({}));
    assert_eq!(
        fixes,
        json!([{
            "title": "fix W001",
            "kind": "quickfix",
            "diagnostic_code": "W001",
            "edits": [],
        }])
    );
    let for_alpha = tool(
        &mut server,
        "specforge.suggest_fixes",
        json!({"entity_id": "alpha"}),
    );
    assert_eq!(for_alpha, fixes);

    // empty_for_clean: beta has no diagnostics.
    let for_beta = tool(
        &mut server,
        "specforge.suggest_fixes",
        json!({"entity_id": "beta"}),
    );
    assert_eq!(for_beta, json!([]));

    assert_tool_invoked(&server, "specforge.suggest_fixes");
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "Provide MCP Format Tool: MCP format tool holds — filesystem_available, files_formatted, check_mode_readonly, mutation_completed_emitted, tool_invoked_emitted"
)]
fn contract_format() {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let messy = root.join("messy.spec");
    let source = "behavior   gamma \"Gamma\"{\ncontract \"x\"\n}\n";
    std::fs::write(&messy, source).unwrap();
    let only_messy = json!(["messy.spec"]);

    // check_mode_readonly: reported, not written, no mutation event.
    let check = tool(
        &mut server,
        "specforge.format",
        json!({"check": true, "paths": only_messy}),
    );
    assert_eq!(check["all_clean"], false, "{check}");
    assert_eq!(check["changed_files"].as_array().unwrap().len(), 1);
    assert!(
        check["changed_files"][0]
            .as_str()
            .unwrap()
            .ends_with("messy.spec")
    );
    assert_eq!(std::fs::read_to_string(&messy).unwrap(), source);
    assert!(events(&server, "mcp_mutation_completed").is_empty());

    // files_formatted: written in the canonical style.
    let write = tool(
        &mut server,
        "specforge.format",
        json!({"paths": only_messy}),
    );
    assert_eq!(write["check_only"], false, "{write}");
    assert_eq!(
        std::fs::read_to_string(&messy).unwrap(),
        "behavior gamma \"Gamma\" {\n  contract \"x\"\n}\n"
    );
    let recheck = tool(
        &mut server,
        "specforge.format",
        json!({"check": true, "paths": only_messy}),
    );
    assert_eq!(recheck["all_clean"], true, "{recheck}");

    // mutation_completed_emitted: once, for the write.
    // It rewrote the one messy file.
    assert_eq!(
        events(&server, "mcp_mutation_completed"),
        [json!({
            "toolName": "specforge.format",
            "files_changed": 1,
            "entities_affected": 0,
            "success": true,
        })]
    );
    assert_tool_invoked(&server, "specforge.format");
}

#[specforge_test(
    behavior = "provide_mcp_extensions_tool",
    verify = "Provide MCP Extensions Tool: MCP extensions tool holds — compiler_api_available, extensions_listed, config_reflected, tool_invoked_emitted"
)]
fn contract_extensions() {
    let dir = project_dir(
        json!({"name":"t","version":"0.1.0","extensions":["@specforge/product", "@acme/absent@1.2.0"]}),
        "",
    );
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": dir.path().to_str().unwrap()}),
    );

    // extensions_listed: name, version, entity kinds and status.
    let listed = tool(&mut server, "specforge.extensions", json!({}));
    let product = find(&listed["extensions"], "name", "@specforge/product");
    assert_eq!(product["status"], "loaded");
    assert!(
        product["version"].as_str().is_some_and(|v| !v.is_empty()),
        "{product}"
    );
    let kinds = product["entity_kinds"].as_array().unwrap();
    assert!(kinds.contains(&json!("Journey")), "{kinds:?}");
    // config_reflected: a configured extension that did not load is listed.
    let absent = find(&listed["extensions"], "name", "@acme/absent");
    assert_eq!(absent["status"], "not_loaded");
    assert_eq!(absent["version"], "1.2.0");
    assert_eq!(absent["entity_kinds"], json!([]));

    // ...and follows specforge.json as it changes.
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name":"t","version":"0.1.0","extensions":["@specforge/product"]}).to_string(),
    )
    .unwrap();
    let listed = tool(&mut server, "specforge.extensions", json!({}));
    let names: Vec<&str> = listed["extensions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["@specforge/product"]);

    assert_tool_invoked(&server, "specforge.extensions");
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "Provide MCP Doctor Tool: MCP doctor tool holds — compiler_api_available, health_checked, resolution_steps_provided, tool_invoked_emitted"
)]
fn contract_doctor() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name":"t","version":"0.1.0","extensions":[]}).to_string(),
    )
    .unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    let installed = tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": product_wasm().to_str().unwrap()}),
    );
    assert_eq!(installed["installed"], true, "{installed}");

    // health_checked: a fresh install is healthy.
    let healthy = tool(&mut server, "specforge.doctor", json!({}));
    assert_eq!(healthy["extensions_ok"], true, "{healthy}");
    assert_eq!(healthy["cache_status"], "ok");

    // Tamper with the installed binary: a stale Wasm cache entry.
    let wasm = walk(&dir.path().join(".specforge").join("extensions"))
        .into_iter()
        .find(|p| p.extension().is_some_and(|e| e == "wasm"))
        .expect("the install wrote a wasm binary");
    std::fs::write(&wasm, b"not the installed module").unwrap();

    let report = tool(&mut server, "specforge.doctor", json!({}));
    assert_eq!(report["extensions_ok"], false, "{report}");
    assert_eq!(report["cache_status"], "stale");
    // resolution_steps_provided: a deterministic step for the issue.
    let finding = find(&report["findings"], "code", "stale_hash");
    assert_eq!(finding["status"], "error");
    assert_eq!(
        finding["remediation"],
        "run `specforge add specforge_ext_product@0.0.0` to reinstall it"
    );
    assert_eq!(tool(&mut server, "specforge.doctor", json!({})), report);

    assert_tool_invoked(&server, "specforge.doctor");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[specforge_test(
    behavior = "expose_graph_as_mcp_resource",
    verify = "Expose Graph as MCP Resource: graph MCP resource holds — validation_complete_fired, graph_json_returned, resource_read_emitted"
)]
fn contract_graph_resource() {
    let mut server = test_server();
    let (content, graph) = resource(&mut server, "specforge://graph");
    assert_eq!(content["uri"], "specforge://graph");
    assert_eq!(content["mimeType"], "application/json");

    // graph_json_returned: the whole graph, schema embedded and versioned.
    assert_eq!(node_ids(&graph), ["alpha", "beta"]);
    assert_eq!(
        graph["edges"],
        json!([{"source": "beta", "target": "alpha", "label": "behaviors"}])
    );
    assert!(graph["schema"].is_object(), "{graph}");
    assert!(graph["schema"]["entity_kinds"].is_array(), "{graph}");
    assert!(graph["schema_version"].is_string(), "{graph}");
    // The same nodes specforge export --format=graph produces.
    let export = tool(&mut server, "specforge.export", json!({"format": "graph"}));
    assert_eq!(graph["nodes"], export["nodes"]);

    assert_resource_read(&server, "specforge://graph");
}

#[specforge_test(
    behavior = "expose_schema_as_mcp_resource",
    verify = "Expose Schema as MCP Resource: schema MCP resource holds — validation_complete_fired, schema_json_returned, resource_read_emitted"
)]
fn contract_schema_resource() {
    let mut server = test_server();
    let (content, schema) = resource(&mut server, "specforge://schema");
    assert_eq!(content["uri"], "specforge://schema");
    assert_eq!(
        schema["entity_kinds"],
        json!({"behavior": ["contract", "verify"], "feature": []})
    );
    assert_eq!(schema["edge_labels"], json!(["behaviors"]));
    assert!(schema["schema_version"].is_string(), "{schema}");

    // Reflects the current compilation: a new kind appears.
    server.state_mut().graph.add_node(node(
        "gamma",
        "invariant",
        span_at("inv.spec", 1, 0, 3),
        text_field("guarantee", "never negative"),
    ));
    let (_, schema) = resource(&mut server, "specforge://schema");
    assert_eq!(schema["entity_kinds"]["invariant"], json!(["guarantee"]));

    assert_resource_read(&server, "specforge://schema");
}

#[specforge_test(
    behavior = "expose_context_as_mcp_resource",
    verify = "Expose Context as MCP Resource: context MCP resource holds — validation_complete_fired, context_format_returned, resource_read_emitted"
)]
fn contract_context_resource() {
    let mut server = test_server();
    let (content, context) = resource(&mut server, "specforge://context");
    assert_eq!(content["uri"], "specforge://context");
    let alpha = find(&context["nodes"], "id", "alpha");
    assert_eq!(alpha["contract"], "MUST work");
    assert_eq!(
        alpha["verify"],
        json!([{"kind": "unit", "description": "works"}])
    );
    assert_eq!(context["edges"].as_array().unwrap().len(), 1);
    // context_format_returned: what export --format=context returns.
    let export = tool(
        &mut server,
        "specforge.export",
        json!({"format": "context"}),
    );
    assert_eq!(context, export);

    assert_resource_read(&server, "specforge://context");
}

#[specforge_test(
    behavior = "expose_brief_as_mcp_resource",
    verify = "Expose Brief as MCP Resource: brief MCP resource holds — validation_complete_fired, brief_format_returned, resource_read_emitted"
)]
fn contract_brief_resource() {
    let mut server = test_server();
    let (content, brief) = resource(&mut server, "specforge://brief");
    assert_eq!(content["uri"], "specforge://brief");
    // brief_format_returned: ids, kinds, titles and edges only.
    assert_eq!(
        brief["nodes"],
        json!([
            {"id": "alpha", "kind": "behavior", "title": "Alpha"},
            {"id": "beta", "kind": "feature", "title": "Beta"},
        ])
    );
    assert_eq!(
        brief["edges"],
        json!([{"source": "beta", "target": "alpha", "label": "behaviors"}])
    );
    let export = tool(&mut server, "specforge.export", json!({"format": "brief"}));
    assert_eq!(brief, export);

    assert_resource_read(&server, "specforge://brief");
}

#[specforge_test(
    behavior = "expose_diagnostics_as_mcp_resource",
    verify = "Expose Diagnostics as MCP Resource: diagnostics MCP resource holds — validation_complete_fired, diagnostics_returned, resource_read_emitted"
)]
fn contract_diagnostics_resource() {
    let mut server = test_server();
    let diagnostics = &mut server.state_mut().diagnostics;
    diagnostics.push(Diagnostic {
        code: "E003".into(),
        severity: Severity::Error,
        message: "unresolved reference 'ghost'".into(),
        span: Some(span_at("feat.spec", 2, 14, 2)),
        suggestion: None,
    });
    diagnostics.push(diagnostic("W001", "a warning", None));

    // diagnostics_returned: severity, code, message, file and position.
    let (content, bag) = resource(&mut server, "specforge://diagnostics");
    assert_eq!(content["uri"], "specforge://diagnostics");
    assert_eq!(
        bag,
        json!([
            {"code": "E003", "severity": "Error", "message": "unresolved reference 'ghost'",
             "file": "feat.spec", "line": 2, "column": 14},
            {"code": "W001", "severity": "Warning", "message": "a warning",
             "suggestion": "fix W001"},
        ])
    );

    // Updates with the compilation's diagnostics.
    server.state_mut().diagnostics.clear();
    let (_, bag) = resource(&mut server, "specforge://diagnostics");
    assert_eq!(bag, json!([]));

    assert_resource_read(&server, "specforge://diagnostics");
}

#[specforge_test(
    behavior = "expose_entity_as_mcp_resource",
    verify = "Expose Per-Entity MCP Resource: per-entity MCP resource holds — validation_complete_fired, subgraph_returned, resource_read_emitted"
)]
fn contract_entity_resource() {
    let mut server = test_server();
    // gamma hangs off beta: two hops from alpha.
    let state = server.state_mut();
    state.graph.add_node(node(
        "gamma",
        "invariant",
        span_at("inv.spec", 1, 0, 3),
        FieldMap::new(),
    ));
    state.graph.add_edge(edge("beta", "gamma", "invariants"));

    // subgraph_returned: alpha, its direct neighbor, the edge between them.
    let (content, entity) = resource(&mut server, "specforge://graph/alpha");
    assert_eq!(content["uri"], "specforge://graph/alpha");
    assert_eq!(node_ids(&entity), ["alpha", "beta"]);
    assert_eq!(
        entity["edges"],
        json!([{"source": "beta", "target": "alpha", "label": "behaviors"}])
    );
    let (_, beta) = resource(&mut server, "specforge://graph/beta");
    assert_eq!(node_ids(&beta), ["alpha", "beta", "gamma"]);

    // An unknown entity is an error, not an empty subgraph.
    let missing = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph/ghost"}),
    );
    assert!(missing["error"].is_object(), "{missing}");

    assert_resource_read(&server, "specforge://graph/alpha");
}

/// Subscribe the default client to `uri`'s delta notifications.
fn subscribe(server: &mut McpServer, uri: &str) {
    let resp = call(server, "resources/subscribe", json!({"uri": uri}));
    assert_eq!(resp["result"], json!({}), "{resp}");
}

// NOT LINKED to "Notify Graph Delta via MCP: graph delta MCP notification
// holds — …": subscribers_notified names the method notifications/graph_changed,
// but the server sends specforge/graphChanged (and many tests pin that name).
#[test]
fn contract_graph_notification() {
    use specforge_mcp::notifications::enqueue_compile_notifications;
    let mut server = test_server();
    let before = server.state().graph.clone();
    server.state_mut().graph.add_node(node(
        "x",
        "behavior",
        span_at("x.spec", 1, 0, 2),
        FieldMap::new(),
    ));

    // No subscriber: the delta is suppressed.
    enqueue_compile_notifications(server.state_mut(), &before, &[]);
    assert!(server.take_notifications().is_empty());
    assert!(events(&server, "mcp_delta_notified").is_empty());

    // A subscriber gets the delta.
    subscribe(&mut server, "specforge://graph");
    enqueue_compile_notifications(server.state_mut(), &before, &[]);
    let delivered = server.take_notifications();
    assert_eq!(delivered.len(), 1, "{delivered:?}");
    assert_eq!(delivered[0]["method"], "specforge/graphChanged");
    assert_eq!(delivered[0]["params"]["added_nodes"], json!(["x"]));
    assert_eq!(delivered[0]["params"]["removed_nodes"], json!([]));
    assert_eq!(
        events(&server, "mcp_delta_notified"),
        [json!({"notificationType": "graph", "subscriberCount": 1,
            "addedNodes": 1, "removedNodes": 0})]
    );

    // An unchanged graph sends nothing.
    let now = server.state().graph.clone();
    enqueue_compile_notifications(server.state_mut(), &now, &[]);
    assert!(server.take_notifications().is_empty());
}

// NOT LINKED to "Notify Diagnostics Delta via MCP: diagnostics delta MCP
// notification holds — …": subscribers_notified names the method
// notifications/diagnostics_changed, but the server sends
// specforge/diagnosticsChanged (and many tests pin that name).
#[test]
fn contract_diagnostics_notification() {
    use specforge_mcp::notifications::enqueue_compile_notifications;
    let mut server = test_server();
    let graph = server.state().graph.clone();
    let old = vec![diagnostic("W001", "old", None)];
    server.state_mut().diagnostics = vec![diagnostic("E003", "new", None)];

    // No subscriber: suppressed.
    enqueue_compile_notifications(server.state_mut(), &graph, &old);
    assert!(server.take_notifications().is_empty());

    subscribe(&mut server, "specforge://diagnostics");
    enqueue_compile_notifications(server.state_mut(), &graph, &old);
    let delivered = server.take_notifications();
    assert_eq!(delivered.len(), 1, "{delivered:?}");
    assert_eq!(delivered[0]["method"], "specforge/diagnosticsChanged");
    assert_eq!(
        delivered[0]["params"]["added"],
        json!([{"code": "E003", "severity": "Warning", "message": "new"}])
    );
    assert_eq!(
        delivered[0]["params"]["removed"],
        json!([{"code": "W001", "severity": "Warning", "message": "old"}])
    );
    let notified = events(&server, "mcp_delta_notified");
    assert_eq!(
        notified,
        [
            json!({"notificationType": "diagnostics", "subscriberCount": 1,
            "addedDiagnostics": 1, "removedDiagnostics": 1})
        ]
    );

    // unchanged_suppressed: the same diagnostics again send nothing.
    let current = server.state().diagnostics.clone();
    enqueue_compile_notifications(server.state_mut(), &graph, &current);
    assert!(server.take_notifications().is_empty());
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "Provide MCP Add Extension Tool: MCP add extension tool holds — filesystem_available, extension_installed, wasm_downloaded, extension_added_emitted, dry_run_safe, tool_invoked_emitted"
)]
fn contract_add_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    // Real offline install of the vendored product blob.
    let blob = product_wasm();
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["installed"], true);
    // Truthful install is observable on disk.
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("specforge_ext_product"));
}

#[test]
fn contract_remove_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    // Removing something that is not installed must refuse.
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@specforge/software"}),
    );
    assert!(resp["error"].is_object());
}

// NOT LINKED to "Provide MCP Migrate Tool: MCP migrate tool holds — …":
// there is only one format version (1.0), so no migration can be applied
// (migrations_applied), and the tool runs no post-migration validation
// (post_migration_validated). What holds today: a current project is left
// alone.
#[test]
fn contract_migrate() {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    let before = std::fs::read_to_string(root.join("test.spec")).unwrap();

    let result = tool(&mut server, "specforge.migrate", json!({"dry_run": false}));
    assert_eq!(result["migrated"], false, "{result}");
    assert_eq!(result["from_version"], result["to_version"]);
    assert_eq!(
        std::fs::read_to_string(root.join("test.spec")).unwrap(),
        before
    );
    assert_tool_invoked(&server, "specforge.migrate");
}

#[specforge_test(
    behavior = "provide_mcp_providers_tool",
    verify = "Provide MCP Providers Tool: MCP providers tool holds — compiler_api_available, providers_listed, tool_invoked_emitted"
)]
fn contract_providers() {
    let mut server = test_server();
    let root = server.state().project_root.clone().unwrap();
    std::fs::write(
        root.join("specforge.json"),
        json!({
            "name": "t", "version": "0.1.0", "extensions": [],
            "providers": [
                {"alias": "github", "scheme": "gh"},
                {"alias": "tracker", "scheme": "jira"},
            ]
        })
        .to_string(),
    )
    .unwrap();
    // Only an extension that contributes providers can back a scheme.
    let github: specforge_registry::ManifestV2 = serde_json::from_value(json!({
        "name": "@acme/github-provider",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "github.wasm",
        "contributes": {"providers": true},
    }))
    .unwrap();
    server.state_mut().manifests.push(github);

    // providers_listed: scheme, alias, backing extension and status.
    let listed = tool(&mut server, "specforge.providers", json!({}));
    assert_eq!(listed["count"], 2);
    assert_eq!(
        find(&listed["providers"], "scheme", "gh"),
        &json!({
            "scheme": "gh", "alias": "github",
            "extension": "@acme/github-provider", "status": "registered",
        })
    );
    assert_eq!(
        find(&listed["providers"], "scheme", "jira")["alias"],
        "tracker"
    );

    assert_tool_invoked(&server, "specforge.providers");
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "Provide MCP Render Tool: MCP render tool holds — graph_available, filesystem_available, files_written, files_listed, tool_invoked_emitted"
)]
fn contract_render() {
    let mut server = test_server();
    let out = tempfile::TempDir::new().unwrap();
    let out_dir = out.path().join("rendered");

    // files_written / files_listed: the json renderer's file, in out_dir.
    let json_render = tool(
        &mut server,
        "specforge.render",
        json!({"format": "json", "out_dir": out_dir.to_str().unwrap()}),
    );
    let graph_file = out_dir.join("graph.json");
    assert_eq!(
        json_render["output_files"],
        json!([graph_file.to_str().unwrap()])
    );
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&graph_file).unwrap()).unwrap();
    assert_eq!(node_ids(&written), ["alpha", "beta"]);

    // The dot renderer writes its own file.
    let dot_render = tool(
        &mut server,
        "specforge.render",
        json!({"format": "dot", "out_dir": out_dir.to_str().unwrap()}),
    );
    let dot_file = out_dir.join("graph.dot");
    assert_eq!(
        dot_render["output_files"],
        json!([dot_file.to_str().unwrap()])
    );
    let dot = std::fs::read_to_string(&dot_file).unwrap();
    assert!(dot.starts_with("digraph"), "{dot}");
    assert!(dot.contains("alpha") && dot.contains("beta"), "{dot}");

    // An unknown format lists the available renderers.
    let unknown = call_tool(&mut server, "specforge.render", json!({"format": "pdf"}));
    assert_eq!(
        unknown["error"]["data"]["available_renderers"],
        json!(["json", "dot", "context", "brief"])
    );

    assert_tool_invoked(&server, "specforge.render");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "Provide MCP Analyze Tool: MCP analyze tool holds — graph_available, passes_run, results_structured, tool_invoked_emitted"
)]
fn contract_analyze() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.analyze", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["ok"].is_boolean(), "analyze must return ok flag");
    let passes = parsed["passes"].as_array().unwrap();
    assert!(!passes.is_empty(), "all-pass run must include passes");
    // Built-in passes always run; coverage is @specforge/testing's pass and
    // this fixture enables no extensions.
    assert!(passes.iter().any(|p| p["pass"] == "contracts"));
}
