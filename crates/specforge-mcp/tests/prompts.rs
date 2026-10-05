use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();

    let mut fields_a = FieldMap::new();
    fields_a.push(
        "contract".into(),
        FieldValue::String("The system MUST do alpha".into()),
    );
    fields_a.push(
        "verify".into(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".into(),
            description: "test alpha".into(),
        }]),
    );

    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: SourceSpan {
            file: "test.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 5,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta Feature".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "features.spec".into(),
            start_line: 10,
            start_col: 0,
            end_line: 15,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    graph.add_node(Node {
        id: EntityId {
            raw: "gamma_orphan".into(),
        },
        kind: EntityKind {
            raw: "invariant".into(),
        },
        title: Some("Gamma".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "inv.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    graph.add_edge(Edge {
        source: "beta".into(),
        target: "alpha".into(),
        label: "behaviors".into(),
    });
    state.serve_graph(graph, Vec::new());
    state.edit_environment(|env| {
        env.registries.kinds.register(kind_entry("behavior", true));
    });
    state.edit_environment(|env| {
        env.registries.kinds.register(kind_entry("invariant", true));
    });
    state.edit_environment(|env| {
        env.registries.kinds.register(kind_entry("feature", false));
    });

    server
}

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
        contract_target: false,
        declares_types: false,
        lifecycle_field: None,
    }
}

fn call_prompt(server: &mut McpServer, name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "prompts/get",
        "params": { "name": name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn prompt_text(resp: &Value) -> String {
    // Data is in the last message (assistant role), instruction is first (user role)
    let messages = resp["result"]["messages"].as_array().unwrap();
    let last = messages.last().unwrap();
    last["content"]["text"].as_str().unwrap().to_string()
}

// --- specforge://prompts/context ---

// B:provide_mcp_context_prompt — verify unit "returns entity context with instructional framing"
#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "specforge://prompts/context returns structured entity context"
)]
fn context_prompt_returns_context() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha"}),
    );
    let messages = resp["result"]["messages"].as_array().unwrap();

    // Must have instruction message + data message
    assert!(
        messages.len() >= 2,
        "prompt must have instruction + data messages, got {}",
        messages.len()
    );

    // First message is instruction (role: user)
    assert_eq!(
        messages[0]["role"], "user",
        "instruction message should be role 'user'"
    );
    let instruction = messages[0]["content"]["text"].as_str().unwrap();
    assert!(
        instruction.contains("implement")
            || instruction.contains("context")
            || instruction.contains("entity"),
        "instruction should guide the agent, got: {}",
        instruction
    );

    // Second message has the data (role: assistant)
    assert_eq!(
        messages[1]["role"], "assistant",
        "data message should be role 'assistant'"
    );
    let text = messages[1]["content"]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert!(parsed["contract_text"].is_string());
    assert!(parsed["upstream_entities"].is_array());
    assert!(parsed["downstream_entities"].is_array());
    assert!(parsed["verify_expectations"].is_array());
}

// B:provide_mcp_context_prompt — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "non-existent entity returns error"
)]
fn context_prompt_unknown_entity() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "nonexistent"}),
    );
    assert!(resp["error"].is_object());
}

// B:provide_mcp_context_prompt — verify unit "includes upstream and downstream"
#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "response includes contract and related entities"
)]
fn context_prompt_includes_edges() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let upstream = parsed["upstream_entities"].as_array().unwrap();
    // beta -> alpha, so beta is upstream of alpha
    assert!(upstream.contains(&json!("beta")));
}

// --- specforge://prompts/review ---

fn review(server: &mut McpServer, args: Value) -> Value {
    let resp = call_prompt(server, "specforge://prompts/review", args);
    serde_json::from_str(&prompt_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

fn reviewed_ids(review: &Value) -> Vec<&str> {
    review["coverage_summary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["entity_id"].as_str().unwrap())
        .collect()
}

fn finding_ids<'a>(review: &'a Value, about: &str) -> Vec<&'a str> {
    review["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["message"].as_str().unwrap().contains(about))
        .map(|f| f["entity_id"].as_str().unwrap())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "specforge://prompts/review returns coverage analysis"
)]
fn review_prompt_analyzes_the_coverage_of_testable_entities() {
    let mut server = test_server();

    let parsed = review(&mut server, json!({}));

    // beta is a feature: not testable, so not reviewed.
    assert_eq!(reviewed_ids(&parsed), ["alpha", "gamma_orphan"], "{parsed}");
    let alpha = &parsed["coverage_summary"][0];
    assert_eq!(alpha["status"], "uncovered", "{parsed}");
    assert_eq!(alpha["declared"], true);
    assert_eq!(alpha["unproven"], json!(["test alpha"]));
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "response identifies entities with missing verification coverage"
)]
fn review_prompt_flags_testable_entities_without_verify() {
    let mut server = test_server();
    let parsed = review(&mut server, json!({}));
    assert_eq!(
        finding_ids(&parsed, "no verify"),
        ["gamma_orphan"],
        "{parsed}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "detects orphan entities"
)]
fn review_prompt_detects_orphans() {
    let mut server = test_server();
    let parsed = review(&mut server, json!({}));
    assert_eq!(
        finding_ids(&parsed, "is an orphan"),
        ["gamma_orphan"],
        "{parsed}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "depth parameter controls neighbor traversal depth"
)]
fn review_depth_bounds_the_neighborhood() {
    let mut server = test_server();
    // alpha <- beta -> delta: delta is two hops from alpha.
    let mut delta = server.state().graph().node("alpha").unwrap().clone();
    delta.id = EntityId {
        raw: "delta".into(),
    };
    server.state_mut().edit_graph(|graph| {
        graph.add_node(delta);
    });
    server.state_mut().edit_graph(|graph| {
        graph.add_edge(Edge {
            source: "beta".into(),
            target: "delta".into(),
            label: "behaviors".into(),
        });
    });

    let default = review(&mut server, json!({"entity_id": "alpha"}));
    assert_eq!(reviewed_ids(&default), ["alpha"], "depth defaults to 1");
    let two = review(&mut server, json!({"entity_id": "alpha", "depth": 2}));
    assert_eq!(reviewed_ids(&two), ["alpha", "delta"]);
    let zero = review(&mut server, json!({"entity_id": "delta", "depth": 0}));
    assert_eq!(reviewed_ids(&zero), ["delta"]);

    let unknown = call_prompt(
        &mut server,
        "specforge://prompts/review",
        json!({"entity_id": "no_such_entity"}),
    );
    assert!(unknown["error"].is_object(), "{unknown}");
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "review prompt returns empty findings when no testable entities exist"
)]
fn review_of_a_graph_without_testable_entities_is_empty() {
    let mut server = test_server();
    // Only beta, a feature with no verify and no edges, is left.
    server.state_mut().edit_graph(|graph| {
        graph.remove_node("alpha");
        graph.remove_node("gamma_orphan");
    });

    let parsed = review(&mut server, json!({}));

    assert_eq!(parsed["findings"], json!([]), "{parsed}");
    assert_eq!(parsed["coverage_summary"], json!([]), "{parsed}");
}

// --- specforge://prompts/trace ---

#[test]
fn trace_prompt_for_an_entity_lists_its_chain() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"entity_id": "alpha"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["affected_entities"].is_array());
    assert!(parsed["unverified_entities"].is_array());
}

#[test]
fn trace_prompt_identifies_unverified() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"entity_id": "alpha"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let unverified = parsed["unverified_entities"].as_array().unwrap();
    // beta is in the trace but has no verify
    assert!(unverified.contains(&json!("beta")));
}

#[test]
fn trace_prompt_unknown_entity() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/trace",
        json!({"entity_id": "nonexistent"}),
    );
    assert!(resp["error"].is_object());
}

/// The trace prompt's result for `plan`, passed as a JSON string the way
/// MCP prompt arguments arrive.
fn trace_plan(server: &mut McpServer, plan: Value) -> Value {
    let resp = call_prompt(
        server,
        "specforge://prompts/trace",
        json!({"plan": plan.to_string()}),
    );
    serde_json::from_str(&prompt_text(&resp)).unwrap_or_else(|_| panic!("{resp}"))
}

fn gap_triples(result: &Value) -> Vec<(String, String, String)> {
    let mut gaps: Vec<(String, String, String)> = result["coverage_gaps"]
        .as_array()
        .unwrap_or_else(|| panic!("no coverage_gaps in {result}"))
        .iter()
        .map(|g| {
            (
                g["source_entity"].as_str().unwrap().to_string(),
                g["target_entity"].as_str().unwrap().to_string(),
                g["missing_link_type"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    gaps.sort();
    gaps
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "specforge://prompts/trace identifies gaps in plan"
)]
fn trace_prompt_finds_the_gaps_in_a_plan() {
    let mut server = test_server();

    // ghost doesn't exist; alpha is testable, has obligations, and is missing.
    let result = trace_plan(
        &mut server,
        json!({"plan_id": "p1", "entries": [
            {"entity_id": "beta", "action": "modify"},
            {"entity_id": "ghost", "action": "create"}
        ]}),
    );

    let owned = |a: &str, b: &str, c: &str| (a.to_string(), b.to_string(), c.to_string());
    assert_eq!(
        gap_triples(&result),
        [
            owned("plan", "alpha", "missing_plan_entry"),
            owned("plan", "ghost", "unresolved_entity"),
        ],
        "{result}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "response returns identified gaps with gap context"
)]
fn trace_prompt_explains_each_gap() {
    let mut server = test_server();

    // beta depends on alpha, yet the plan does beta first.
    let result = trace_plan(
        &mut server,
        json!({"entries": [{"entity_id": "beta"}, {"entity_id": "alpha"}]}),
    );

    let gaps = result["coverage_gaps"].as_array().unwrap();
    assert_eq!(gaps.len(), 1, "{result}");
    assert_eq!(gaps[0]["missing_link_type"], "ordering");
    assert_eq!(
        gaps[0]["gap_context"],
        "'beta' depends on 'alpha' (via behaviors), but 'alpha' appears later in the plan"
    );
    let again = trace_plan(
        &mut server,
        json!({"entries": [{"entity_id": "beta"}, {"entity_id": "alpha"}]}),
    );
    assert_eq!(result, again, "gap context is deterministic");
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "affected entities are listed"
)]
fn trace_prompt_lists_the_entities_a_plan_affects() {
    let mut server = test_server();

    let result = trace_plan(&mut server, json!({"entries": [{"entity_id": "beta"}]}));

    // beta and what its chain reaches; gamma_orphan is untouched.
    assert_eq!(
        result["affected_entities"],
        json!(["alpha", "beta"]),
        "{result}"
    );
    assert_eq!(result["unverified_entities"], json!(["beta"]), "{result}");
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "malformed plan JSON returns validation error"
)]
fn trace_prompt_rejects_a_malformed_plan() {
    let mut server = test_server();
    let error = |server: &mut McpServer, plan: &str| {
        let resp = call_prompt(server, "specforge://prompts/trace", json!({"plan": plan}));
        resp["error"]["message"]
            .as_str()
            .unwrap_or_else(|| panic!("no error for {plan}: {resp}"))
            .to_string()
    };

    assert!(error(&mut server, "{not json").contains("not valid JSON"));
    assert!(error(&mut server, "[1, 2]").contains("entries"));
    let message = error(
        &mut server,
        r#"{"entries": [{"entity_id": "beta"}, {"id": "x"}]}"#,
    );
    assert!(message.contains("entries[1].entity_id"), "{message}");
}

// --- specforge://prompts/explore ---

// B:provide_mcp_explore_prompt — verify unit "returns exploration data"
#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "specforge://prompts/explore returns exploration starting points"
)]
fn explore_prompt_returns_data() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Starting points rank by out-degree minus in-degree: beta (1 out) is the
    // top-down entry, gamma_orphan (0) next, alpha (1 in) last.
    assert_eq!(
        parsed["starting_points"],
        json!(["beta", "gamma_orphan", "alpha"])
    );
}

// B:provide_mcp_explore_prompt — verify unit "identifies orphan nodes"
#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "orphan_nodes field lists entities with zero incoming and outgoing edges"
)]
fn explore_prompt_identifies_orphans() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let orphans = parsed["orphan_nodes"].as_array().unwrap();
    assert!(orphans.contains(&json!("gamma_orphan")));
}

// B:provide_mcp_explore_prompt — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "kind filter restricts results to matching entity kind"
)]
fn explore_prompt_kind_filter() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/explore",
        json!({"kind": "behavior"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let matching = parsed["matching_entities"].as_array().unwrap();
    assert!(matching.contains(&json!("alpha")));
    assert!(!matching.contains(&json!("beta")));
}

// Unknown prompt
#[specforge_test(
    behavior = "mcp_structured_error_responses",
    verify = "no MCP endpoint returns a plain string error"
)]
fn unknown_prompt_returns_error() {
    let mut server = test_server();
    // A failing call on every endpoint family: prompts, tools, resources,
    // subscriptions, and an unknown method.
    let failing_calls = [
        (
            "prompts/get",
            json!({"name": "specforge://prompts/nonexistent"}),
        ),
        (
            "prompts/get",
            json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "nope"}}),
        ),
        ("prompts/get", json!({})),
        ("tools/call", json!({"name": "specforge.nonexistent"})),
        ("tools/call", json!({})),
        ("resources/read", json!({"uri": "specforge://nonexistent"})),
        ("resources/read", json!({"uri": "specforge://graph/"})),
        ("resources/read", json!({})),
        ("resources/subscribe", json!({})),
        ("no/such/method", json!({})),
    ];
    for (method, params) in failing_calls {
        let req = json!({"jsonrpc": "2.0", "id": 7, "method": method, "params": params});
        let resp: Value =
            serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
        let error = &resp["error"];
        assert!(
            error.is_object(),
            "{method} {params}: error must be an object, got {resp}"
        );
        assert!(
            error["code"].is_i64(),
            "{method} {params}: error.code must be an integer, got {error}"
        );
        let message = error["message"].as_str().unwrap_or_default();
        assert!(
            !message.is_empty(),
            "{method} {params}: error.message must be a non-empty string, got {error}"
        );
        assert!(resp.get("result").is_none(), "{method}: {resp}");
        // A prompt that cannot render carries its McpError as data.
        if method == "prompts/get" && params["arguments"]["entity_id"] == "nope" {
            assert_eq!(error["data"]["code"], "entity_not_found", "{resp}");
        }
    }
}

// Prompt when not initialized
#[test]
fn prompt_not_initialized() {
    let mut server = McpServer::new();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha"}),
    );
    assert!(resp["error"].is_object());
}

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".into(),
        start_line: 1,
        start_col: 0,
        end_line: 5,
        end_col: 0,
    }
}

// B:provide_mcp_context_prompt — verify unit "context prompt works with zero extensions installed"
#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "context prompt works with zero extensions installed"
)]
fn context_zero_extensions() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId {
            raw: "minimal".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Minimal".into()),
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    });
    state.serve_graph(graph, Vec::new());

    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "minimal"}),
    );
    assert!(
        server.state().registries().kinds.is_empty(),
        "no extension may be installed"
    );
    let parsed: Value = serde_json::from_str(&prompt_text(&resp)).unwrap();
    assert_eq!(parsed["entity_id"], "minimal");
    assert_eq!(parsed["kind"], "behavior");
    assert_eq!(parsed["upstream_entities"], json!([]));
    assert_eq!(parsed["downstream_entities"], json!([]));
}

// B:provide_mcp_explore_prompt — verify unit "entity_id focuses exploration on that entity"
#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "entity_id focuses exploration on that entity"
)]
fn explore_entity_id_focus() {
    let mut server = test_server();
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/explore",
        json!({"entity_id": "alpha"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // beta and gamma_orphan match with no filter; the focus drops them.
    assert_eq!(parsed["matching_entities"], json!(["alpha"]));
}

// B:provide_mcp_explore_prompt — verify unit "high_connectivity excludes zero-edge nodes"
#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "high_connectivity field lists entities with highest edge degree"
)]
fn explore_high_connectivity() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let high_conn = parsed["high_connectivity"].as_array().unwrap();
    // gamma_orphan has zero edges — must NOT appear in high_connectivity
    assert!(
        !high_conn.contains(&json!("gamma_orphan")),
        "zero-edge nodes must not appear in high_connectivity, got: {:?}",
        high_conn
    );
    // alpha and beta have edges — they should be in high_connectivity
    assert!(
        high_conn.contains(&json!("alpha")),
        "alpha (has edges) should be in high_connectivity"
    );
    assert!(
        high_conn.contains(&json!("beta")),
        "beta (has edges) should be in high_connectivity"
    );
}

// B:provide_mcp_context_prompt — verify unit "context includes contract text"
#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "response includes contract and related entities"
)]
fn context_includes_contract() {
    let mut server = test_server();
    crate::support::declare_headline_fields(&mut server, "behavior");
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "alpha"}),
    );
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["contract_text"].is_string());
    assert!(parsed["contract_text"].as_str().unwrap().contains("MUST"));
}

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "context includes every field, like an invariant's guarantee"
)]
fn context_includes_every_field() {
    let mut server = test_server();
    let mut fields = FieldMap::new();
    fields.push(
        "guarantee".into(),
        FieldValue::String("Ids MUST be unique".into()),
    );
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "unique_ids".into(),
            },
            kind: EntityKind {
                raw: "invariant".into(),
            },
            title: None,
            fields,
            source_span: SourceSpan {
                file: "t.spec".into(),
                start_line: 1,
                start_col: 1,
                end_line: 3,
                end_col: 2,
            },
            methods: Vec::new(),
        });
    });
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "unique_ids"}),
    );
    let parsed: Value = serde_json::from_str(&prompt_text(&resp)).unwrap();
    assert_eq!(
        parsed["fields"]["guarantee"], "Ids MUST be unique",
        "{parsed}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "review coverage matches specforge.coverage obligation by obligation"
)]
fn review_coverage_matches_the_coverage_tool() {
    let mut server = test_server();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge-report.json"),
        r#"{"results":{"alpha":{"tests":[{"name":"t","verify":"test alpha","status":"pass"}]}}}"#,
    )
    .unwrap();
    crate::support::serve_in_memory_at(server.state_mut(), project.path());
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/review",
        json!({"entity_id": "alpha"}),
    );
    let parsed: Value = serde_json::from_str(&prompt_text(&resp)).unwrap();
    let alpha = &parsed["coverage_summary"][0];
    assert_eq!(alpha["status"], "covered", "{parsed}");
    assert_eq!(alpha["linked"], true);
}

// --- The Prompt spec pipeline (serve_mcp_prompt) ---

/// A full, valid argument set for each core prompt over `test_server`'s
/// graph: every argument the prompt lists.
fn full_arguments(prompt: &str) -> Value {
    match prompt {
        "specforge://prompts/context" => {
            json!({"entity_id": "alpha", "structural_constraints": "gamma_orphan"})
        }
        "specforge://prompts/review" => json!({"entity_id": "alpha", "depth": "1"}),
        "specforge://prompts/trace" => json!({
            "plan": {"entries": [{"entity_id": "alpha", "action": "modify"}]},
            "entity_id": "alpha",
        }),
        "specforge://prompts/explore" => json!({"entity_id": "alpha", "kind": "behavior"}),
        "specforge://prompts/infer" => {
            json!({"scope": "plan", "target_spec_directory": "spec/", "cursor": "0"})
        }
        other => panic!("no full argument set for {other}"),
    }
}

/// Each listed prompt and its listed arguments, `(name, required)`.
fn listed_prompts(server: &mut McpServer) -> Vec<(String, Vec<(String, bool)>)> {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "prompts/list", "params": {}});
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    resp["result"]["prompts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let arguments = p["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| {
                    (
                        a["name"].as_str().unwrap().to_string(),
                        a["required"].as_bool().unwrap(),
                    )
                })
                .collect();
            (p["name"].as_str().unwrap().to_string(), arguments)
        })
        .collect()
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "a listed required argument is exactly one the prompt cannot render without"
)]
fn listed_required_arguments_are_exactly_the_unrenderable_omissions() {
    let mut server = test_server();
    for (prompt, arguments) in listed_prompts(&mut server) {
        let full = full_arguments(&prompt);
        let listed: Vec<&str> = arguments.iter().map(|(name, _)| name.as_str()).collect();
        let given: Vec<&str> = full
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let (mut listed_sorted, mut given_sorted) = (listed.clone(), given.clone());
        listed_sorted.sort_unstable();
        given_sorted.sort_unstable();
        assert_eq!(
            listed_sorted, given_sorted,
            "{prompt}: the full set is every listed argument"
        );

        let rendered = call_prompt(&mut server, &prompt, full.clone());
        assert!(
            rendered["error"].is_null(),
            "{prompt} renders with every argument: {rendered}"
        );

        for (argument, required) in &arguments {
            let mut without = full.clone();
            without.as_object_mut().unwrap().remove(argument);
            let resp = call_prompt(&mut server, &prompt, without);
            let missing = resp["error"]["message"]
                .as_str()
                .is_some_and(|m| m.starts_with("Missing required parameter"));
            if *required {
                assert_eq!(
                    resp["error"]["code"], -32602,
                    "{prompt} without {argument}: {resp}"
                );
                assert_eq!(
                    resp["error"]["data"]["argument"],
                    argument.as_str(),
                    "{resp}"
                );
                assert!(missing, "{prompt} without {argument}: {resp}");
            } else {
                assert!(!missing, "{prompt} without optional {argument}: {resp}");
            }
        }
    }
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "a prompt refusal is a JSON-RPC error whose data is an McpError naming the prompt"
)]
#[specforge_test(
    behavior = "mcp_structured_error_responses",
    verify = "a failed prompts/get carries its McpError as the error's data"
)]
fn every_prompt_refusal_carries_an_mcp_error() {
    let mut server = test_server();
    for (name, args) in [
        ("context", json!({})),
        ("context", json!({"entity_id": "ghost"})),
        ("context", json!({"entity_id": 42})),
        ("review", json!({"entity_id": "ghost"})),
        ("review", json!({"entity_id": "alpha", "depth": "two"})),
        ("trace", json!({})),
        ("trace", json!({"plan": "{not json"})),
        ("trace", json!({"entity_id": "ghost"})),
        ("infer", json!({"scope": "kind:"})),
        ("infer", json!({"scope": "kind:nope"})),
        ("infer", json!({"scope": "plan", "cursor": "-1"})),
    ] {
        let prompt = format!("specforge://prompts/{name}");
        let resp = call_prompt(&mut server, &prompt, args.clone());
        let error = &resp["error"];
        let data = &error["data"];
        assert!(data["code"].is_string(), "{name} {args}: {resp}");
        assert_eq!(data["prompt"], prompt.as_str(), "{resp}");
        assert!(data.get("tool").is_none(), "{resp}");
        assert_eq!(error["message"], data["message"], "{resp}");
        let expected = if data["code"] == "invalid_input" || data["code"] == "entity_not_found" {
            -32602
        } else {
            -32603
        };
        assert_eq!(error["code"], expected, "{resp}");
    }
    // An unknown entity is the tools' entity_not_found, its E003 in diagnostic.
    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "ghost"}),
    );
    let data = &resp["error"]["data"];
    assert_eq!(data["code"], "entity_not_found", "{resp}");
    assert_eq!(data["entity_id"], "ghost");
    assert_eq!(data["diagnostic"]["code"], "E003");
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "a missing required prompt argument is -32602 naming the argument"
)]
fn missing_required_prompt_argument_names_it() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");
    assert_eq!(
        resp["error"]["message"],
        "Missing required parameter: entity_id"
    );
    let data = &resp["error"]["data"];
    assert_eq!(data["code"], "invalid_input");
    assert_eq!(data["argument"], "entity_id");
    assert_eq!(data["prompt"], "specforge://prompts/context");
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "prompt arguments that are not an object produce -32602 Invalid params"
)]
fn prompt_arguments_must_be_an_object() {
    let mut server = test_server();
    for arguments in [json!("x"), json!(["entity_id"]), json!(3)] {
        let resp = call_prompt(
            &mut server,
            "specforge://prompts/context",
            arguments.clone(),
        );
        assert_eq!(resp["error"]["code"], -32602, "{arguments}: {resp}");
        assert_eq!(
            resp["error"]["message"], "Invalid params: arguments must be an object",
            "{resp}"
        );
    }
    // Absent or null arguments are none.
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": "prompts/get",
        "params": {"name": "specforge://prompts/explore", "arguments": null}});
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    assert!(resp["error"].is_null(), "{resp}");
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "a numeric prompt argument is read from a string, as MCP sends it"
)]
fn review_reads_depth_from_a_string() {
    let mut server = test_server();
    // alpha <- beta -> delta: delta, testable, is two hops from alpha.
    let mut delta = server.state().graph().node("alpha").unwrap().clone();
    delta.id = EntityId {
        raw: "delta".into(),
    };
    server.state_mut().edit_graph(|graph| {
        graph.add_node(delta);
        graph.add_edge(Edge {
            source: "beta".into(),
            target: "delta".into(),
            label: "behaviors".into(),
        });
    });
    let as_number = review(&mut server, json!({"entity_id": "alpha", "depth": 2}));
    let as_string = review(&mut server, json!({"entity_id": "alpha", "depth": "2"}));
    assert_eq!(as_string, as_number);
    assert_eq!(reviewed_ids(&as_string), ["alpha", "delta"]);
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "a numeric prompt argument is read from a string, as MCP sends it"
)]
fn infer_plan_reads_cursor_from_a_string() {
    let mut server = test_server();
    let plan = |server: &mut McpServer, cursor: Value| {
        let resp = call_prompt(
            server,
            "specforge://prompts/infer",
            json!({"scope": "plan", "cursor": cursor}),
        );
        let text = resp["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .split("## Reference Data\n")
            .nth(1)
            .map(str::to_string)
            .unwrap_or_default();
        serde_json::from_str::<Value>(&text).unwrap()["plan"]["cursor"].clone()
    };
    assert_eq!(plan(&mut server, json!("50")), 50);
    assert_eq!(plan(&mut server, json!(50)), 50);
}

#[specforge_test(
    behavior = "serve_mcp_prompt",
    verify = "an unknown prompt records no mcp_prompt_invoked event"
)]
fn unknown_prompt_records_no_invocation() {
    let mut server = test_server();
    let invoked = |server: &McpServer| {
        server
            .state()
            .events
            .iter()
            .filter(|e| e.name == "mcp_prompt_invoked")
            .count()
    };
    let resp = call_prompt(&mut server, "specforge://prompts/nope", json!({}));
    assert_eq!(resp["error"]["code"], -32602, "{resp}");
    assert_eq!(invoked(&server), 0);
    // A known prompt refused for its arguments is still an invocation, as a
    // tool's is.
    call_prompt(&mut server, "specforge://prompts/context", json!({}));
    assert_eq!(invoked(&server), 1);
}
