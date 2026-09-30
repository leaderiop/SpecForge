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
    state.graph = graph;
    state.kind_registry.register(kind_entry("behavior", true));
    state.kind_registry.register(kind_entry("invariant", true));
    state.kind_registry.register(kind_entry("feature", false));

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
    let mut delta = server.state().graph.node("alpha").unwrap().clone();
    delta.id = EntityId {
        raw: "delta".into(),
    };
    server.state_mut().graph.add_node(delta);
    server.state_mut().graph.add_edge(Edge {
        source: "beta".into(),
        target: "delta".into(),
        label: "behaviors".into(),
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
    server.state_mut().graph.remove_node("alpha");
    server.state_mut().graph.remove_node("gamma_orphan");

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
    assert!(parsed["matching_entities"].is_array());
    assert!(parsed["starting_points"].is_array());
    assert!(parsed["high_connectivity"].is_array());
    assert!(parsed["orphan_nodes"].is_array());
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
    let resp = call_prompt(&mut server, "specforge://prompts/nonexistent", json!({}));
    assert!(resp["error"].is_object());
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
    state.graph = graph;

    let resp = call_prompt(
        &mut server,
        "specforge://prompts/context",
        json!({"entity_id": "minimal"}),
    );
    assert!(resp["result"]["messages"].is_array());
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
    let matching = parsed["matching_entities"].as_array().unwrap();
    assert!(matching.contains(&json!("alpha")));
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
    server.state_mut().graph.add_node(Node {
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
    server.state_mut().project_root = Some(project.path().to_path_buf());
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
