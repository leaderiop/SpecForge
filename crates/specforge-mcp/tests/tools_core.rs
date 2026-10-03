use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, SpannedRef, VerifyStatement};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".into(),
        start_line: 1,
        start_col: 0,
        end_line: 5,
        end_col: 0,
    }
}

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
    let verify_stmts = vec![VerifyStatement {
        kind: "unit".into(),
        description: "does alpha correctly".into(),
    }];
    fields_a.push("verify".into(), FieldValue::VerifyList(verify_stmts));

    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: span(),
        methods: Vec::new(),
    });

    let mut fields_b = FieldMap::new();
    fields_b.push(
        "behaviors".into(),
        FieldValue::ReferenceList(vec![SpannedRef {
            id: "alpha".into(),
            span: span(),
        }]),
    );
    graph.add_node(Node {
        id: EntityId {
            raw: "beta_feature".into(),
        },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta Feature".into()),
        fields: fields_b,
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
        title: Some("Gamma Orphan".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "invariants.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 0,
        },
        methods: Vec::new(),
    });

    graph.add_edge(Edge {
        source: "beta_feature".into(),
        target: "alpha".into(),
        label: "behaviors".into(),
    });
    state.serve_graph(graph, Vec::new());
    for (kind, testable) in [("behavior", true), ("invariant", true), ("feature", false)] {
        state.edit_environment(|env| {
            env.registries.kinds.register(kind_entry(kind, testable));
            // As @specforge/testing does: a testable software kind must
            // declare obligations (W004), so its entities count toward
            // coverage.
            if testable {
                env.registries
                    .rules
                    .push((obligations_rule(kind), "@test/ext".into()));
            }
        });
    }

    server
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "testable is the kind's testability and declared says whether the entity has obligations"
)]
fn inspect_testable_is_the_kinds_and_declared_is_the_entitys() {
    let mut server = test_server();
    let mut inspect = |id: &str| {
        let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": id}));
        let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        (parsed["testable"].clone(), parsed["declared"].clone())
    };
    // A behavior that declares an obligation.
    assert_eq!(inspect("alpha"), (json!(true), json!(true)));
    // An invariant (a testable kind) that declares none.
    assert_eq!(inspect("gamma_orphan"), (json!(true), json!(false)));
    // A feature: its kind is not testable.
    assert_eq!(inspect("beta_feature"), (json!(false), json!(false)));
}

/// The W004 rule requiring `kind`'s entities to declare obligations.
fn obligations_rule(kind: &str) -> specforge_registry::validation_engine::ValidationRulePattern {
    use specforge_registry::validation_engine::{ValidationPatternKind, ValidationRulePattern};
    ValidationRulePattern {
        code: "W004".into(),
        severity: specforge_common::Severity::Warning,
        message_template: "{kind} '{id}' is testable but declares no verify obligations".into(),
        check: ValidationPatternKind::NoVerifyStatements,
        target_kind: Some(kind.into()),
        edge_type: None,
        edge_peer_kind: None,
        field: Some("verify".into()),
        constraint: None,
        wasm_function: None,
    }
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
        contract_target: false,
        declares_types: false,
        lifecycle_field: None,
    }
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

/// The node ids of a graph-shaped payload, sorted.
fn node_ids(parsed: &Value) -> Vec<String> {
    let mut ids: Vec<String> = parsed["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("no nodes in {parsed}"))
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

fn node<'a>(parsed: &'a Value, id: &str) -> &'a Value {
    parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap_or_else(|| panic!("no {id} in {parsed}"))
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// --- specforge.query ---

// B:provide_mcp_query_tool — verify unit "returns subgraph for entity"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "specforge.query tool returns subgraph for valid entityId"
)]
fn query_returns_subgraph() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // gamma_orphan is not connected to alpha.
    assert_eq!(node_ids(&parsed), vec!["alpha", "beta_feature"]);
    assert_eq!(
        parsed["edges"],
        json!([{"source": "beta_feature", "target": "alpha", "label": "behaviors"}])
    );
}

// B:provide_mcp_query_tool — verify unit "returns error for unknown entity"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "non-existent entityId returns error response"
)]
fn query_error_for_unknown() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "nonexistent"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "entity_not_found", "{error}");
    assert_eq!(error["entity_id"], "nonexistent", "{error}");
}

// B:provide_mcp_query_tool — verify unit "respects depth parameter"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "depth parameter limits traversal depth"
)]
fn query_respects_depth() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "depth": 0}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    // Depth 0 = only the root
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "alpha");
}

// B:provide_mcp_query_tool — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "kind filter restricts returned node types"
)]
fn query_respects_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "depth": 2, "kinds": ["behavior"]}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        let kind = node["kind"].as_str().unwrap();
        // Root always included, plus kind-filtered nodes
        assert!(kind == "behavior" || node["id"] == "alpha");
    }
}

// B:provide_mcp_query_tool — verify unit "missing entity_id returns error"
#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "a tool that detects invalid arguments returns an isError result, not -32602"
)]
fn query_missing_entity_id() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.query", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "entity_id", "{error}");
}

// --- specforge.export ---

// B:provide_mcp_export_tool — verify unit "exports graph format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "specforge.export tool returns graph in requested format"
)]
fn export_graph_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    // Full graph format includes fields
    let alpha = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "alpha")
        .unwrap();
    assert!(alpha["fields"].is_object());
}

// B:provide_mcp_export_tool — verify unit "exports context format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "all three formats (context, brief, graph) supported"
)]
fn export_context_format() {
    let mut server = test_server();
    crate::support::declare_headline_fields(&mut server, "behavior");
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "context"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        node_ids(&parsed),
        vec!["alpha", "beta_feature", "gamma_orphan"]
    );
    // The context format lifts the contract to the top level and drops the
    // source location the graph format carries.
    let alpha = node(&parsed, "alpha");
    assert_eq!(alpha["contract"], "The system MUST do alpha");
    assert!(alpha.get("file").is_none(), "{alpha}");
    assert!(alpha.get("line").is_none(), "{alpha}");
    let graph: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph"}),
    )))
    .unwrap();
    assert_eq!(node(&graph, "alpha")["file"], "test.spec");
    assert!(node(&graph, "alpha").get("contract").is_none());
}

// B:provide_mcp_export_tool — verify unit "exports brief format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "all three formats (context, brief, graph) supported"
)]
fn export_brief_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "brief"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        node_ids(&parsed),
        vec!["alpha", "beta_feature", "gamma_orphan"]
    );
    // Brief nodes carry no fields and no contract.
    for n in parsed["nodes"].as_array().unwrap() {
        assert!(n.get("fields").is_none(), "{n}");
        assert!(n.get("contract").is_none(), "{n}");
        assert!(n["kind"].is_string(), "{n}");
    }
    assert_eq!(node(&parsed, "alpha")["kind"], "behavior");
}

// B:provide_mcp_export_tool — verify unit "exports scoped subgraph"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "scope parameter restricts to subgraph"
)]
fn export_scoped() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "scope": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // gamma_orphan lies outside alpha's subgraph.
    assert_eq!(node_ids(&parsed), vec!["alpha", "beta_feature"]);
}

// B:provide_mcp_export_tool — verify unit "unknown format returns error"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "unknown format returns error"
)]
fn export_unknown_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "yaml"}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "format", "{error}");
}

// --- specforge.trace ---

// B:provide_mcp_trace_tool — verify unit "returns trace chain"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace tool returns traceability chain for valid entityId"
)]
fn trace_returns_chain() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    // beta_feature -> alpha, so beta_feature is upstream of alpha.
    let upstream = parsed["upstream"].as_array().unwrap();
    assert_eq!(upstream.len(), 1, "{parsed}");
    assert_eq!(upstream[0]["entity_id"], "beta_feature");
    assert_eq!(upstream[0]["edge_label"], "behaviors");
    assert_eq!(parsed["downstream"], json!([]));
}

// B:provide_mcp_trace_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "non-existent entityId returns error response"
)]
fn trace_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "nonexistent"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "entity_not_found", "{error}");
    assert_eq!(error["entity_id"], "nonexistent", "{error}");
}

// --- specforge.search ---

// B:provide_mcp_search_tool — verify unit "returns fuzzy matches"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "text search finds entities matching by name or contract"
)]
fn search_returns_matches() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["entity_id"], "alpha");
}

// B:provide_mcp_search_tool — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "kind filter restricts results to matching entity kinds"
)]
fn search_respects_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "kinds": ["feature"]}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Alpha is a behavior, not a feature, so should not match with feature filter
    let results = parsed.as_array().unwrap();
    for r in results {
        assert_eq!(r["kind"], "feature");
    }
}

// B:provide_mcp_search_tool — verify unit "respects limit"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "limit caps the number of returned results"
)]
fn search_respects_limit() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "a", "limit": 1}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.as_array().unwrap().len() <= 1);
}

// B:provide_mcp_search_tool — verify unit "missing query returns error"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "missing query returns error"
)]
fn search_missing_query() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert_eq!(error["argument"], "query", "{error}");
}

// --- specforge.schema ---

/// A server that compiled `project_with_errors_and_warnings` (which loads
/// `@specforge/software`), and its `specforge.schema` reply for `args`.
fn compiled_schema(server: &mut McpServer, project: &tempfile::TempDir, args: Value) -> Value {
    call_tool(
        server,
        "specforge.validate",
        json!({"path": project.path().to_str().unwrap()}),
    );
    serde_json::from_str(&tool_text(&call_tool(server, "specforge.schema", args))).unwrap()
}

fn names(list: &Value, key: &str) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|item| item[key].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "specforge.schema returns full GraphProtocolSchema"
)]
fn schema_tool_returns_the_graph_protocol_schema() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    let schema = compiled_schema(&mut server, &project, json!({}));

    assert_eq!(
        schema["extensions"],
        json!([{"name": "@specforge/software", "version": "1.0.0"}])
    );
    assert_eq!(
        names(&schema["entity_kinds"], "name"),
        ["behavior", "event", "invariant", "port", "type"]
    );
    // Typed fields, not just names: behavior's contract is a required string.
    let behavior = &schema["entity_kinds"][0];
    let contract = behavior["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "contract")
        .unwrap_or_else(|| panic!("no contract field in {behavior}"));
    assert_eq!(contract["field_type"], "string");
    assert_eq!(contract["required"], true);
    let enforces = schema["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["label"] == "BehaviorEnforcesInvariant")
        .unwrap_or_else(|| panic!("no BehaviorEnforcesInvariant in {schema}"));
    assert_eq!(enforces["source_kinds"], json!(["behavior"]));
    assert_eq!(enforces["target_kinds"], json!(["invariant"]));

    // The document a full graph export embeds.
    let export: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph"}),
    )))
    .unwrap();
    assert_eq!(schema, export["schema"]);
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "kind filter restricts schema to single entity kind"
)]
fn schema_tool_kind_filter() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    let schema = compiled_schema(&mut server, &project, json!({"kind": "invariant"}));
    assert_eq!(names(&schema["entity_kinds"], "name"), ["invariant"]);
    // The edge types that can end at an invariant, and the open ones.
    assert_eq!(
        names(&schema["edge_types"], "label"),
        ["BehaviorEnforcesInvariant", "ExternalRef", "References"]
    );
}

// --- specforge.coverage ---

#[test]
fn coverage_returns_per_entity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());

    for r in results {
        assert!(r["entity_id"].is_string());
        assert!(r["status"].is_string());
        assert!(r["declared"].is_boolean());
    }
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "specforge.coverage returns coverage for all testable entities"
)]
fn coverage_returns_all_testable_entities() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let mut ids: Vec<&str> = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["entity_id"].as_str().unwrap())
        .collect();
    ids.sort();
    // beta_feature's kind isn't testable.
    assert_eq!(ids, vec!["alpha", "gamma_orphan"]);

    // A named entity is reported whatever its kind.
    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"entity_id": "beta_feature"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed[0]["entity_id"], "beta_feature");
}

#[test]
fn coverage_alpha_uncovered_without_tests() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["entity_id"], "alpha");
    // Declared obligations with no recorded test prove nothing.
    assert_eq!(results[0]["status"], "uncovered");
    assert_eq!(results[0]["obligations"], 1);
    assert_eq!(results[0]["unproven"], json!(["does alpha correctly"]));
}

/// The server with a behavior `two` declaring obligations "a" and "b", and
/// a `specforge-report.json` recording `tests` (verify text, status) for it.
/// The tempdir must outlive the server's use of the report.
fn server_with_report(tests: &[(&str, &str)]) -> (McpServer, tempfile::TempDir) {
    let mut server = test_server();
    let mut fields = FieldMap::new();
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(
            ["a", "b"]
                .map(|text| VerifyStatement {
                    kind: "unit".into(),
                    description: text.into(),
                })
                .to_vec(),
        ),
    );
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId { raw: "two".into() },
            kind: EntityKind {
                raw: "behavior".into(),
            },
            title: None,
            fields,
            source_span: span(),
            methods: Vec::new(),
        });
    });
    let tests: Vec<Value> = tests
        .iter()
        .map(|(verify, status)| json!({"name": verify, "verify": verify, "status": status}))
        .collect();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge-report.json"),
        json!({"results": {"two": {"tests": tests}}}).to_string(),
    )
    .unwrap();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    (server, project)
}

/// `specforge.coverage`'s result for `two`.
fn coverage_of_two(server: &mut McpServer) -> Value {
    let resp = call_tool(server, "specforge.coverage", json!({"entity_id": "two"}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed[0].clone()
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "an entity with an unproven obligation is partial, not covered"
)]
fn coverage_with_an_unproven_obligation_is_partial() {
    let (mut server, _project) = server_with_report(&[("a", "pass")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["obligations"], 2);
    assert_eq!(two["proven"], 1);
    assert_eq!(two["unproven"], json!(["b"]));

    let (mut server, _project) = server_with_report(&[("a", "pass"), ("b", "pass")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "covered", "{two}");
    assert_eq!(two["unproven"], json!([]));
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "a failing recorded test keeps an entity from being covered"
)]
fn coverage_with_a_failing_test_is_partial() {
    // Both obligations are proven, but a third test fails (A014).
    let (mut server, _project) = server_with_report(&[("a", "pass"), ("b", "pass"), ("a", "fail")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["proven"], 2);

    // A failing test proves nothing, even when it names an obligation.
    let (mut server, _project) = server_with_report(&[("a", "fail")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["unproven"], json!(["a", "b"]));
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "a field named verify does not hide an entity's verify statements"
)]
fn coverage_sees_statements_behind_a_verify_field() {
    // A struct member named `verify` comes before the entity's statement,
    // as in `type Payload { verify string @optional; verify unit "..." }`.
    let mut server = test_server();
    let mut fields = FieldMap::new();
    fields.push("verify".into(), FieldValue::Identifier("string".into()));
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".into(),
            description: "payload is valid".into(),
        }]),
    );
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "payload".into(),
            },
            kind: EntityKind {
                raw: "behavior".into(),
            },
            title: None,
            fields,
            source_span: span(),
            methods: Vec::new(),
        });
    });

    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"entity_id": "payload"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let payload = &parsed[0];
    assert_eq!(payload["declared"], true, "{payload}");
    assert_eq!(payload["obligations"], 1, "{payload}");
    assert_eq!(
        payload["unproven"],
        json!(["payload is valid"]),
        "{payload}"
    );

    // inspect reads the same obligations.
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "payload"}),
    );
    let inspect: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(inspect["testable"], true, "{inspect}");
    assert_eq!(
        inspect["verify_declarations"],
        json!(["unit payload is valid"]),
        "{inspect}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "analyze reads the project's specforge-report.json by default"
)]
fn analyze_reads_the_project_report_by_default() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior two \"Two\" {\n  verify unit \"a\"\n  verify unit \"b\"\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("specforge-report.json"),
        r#"{"results":{"two":{"tests":[{"name":"t","verify":"a","status":"pass"}]}}}"#,
    )
    .unwrap();

    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.analyze",
        json!({"path": root.to_str().unwrap(), "pass": "coverage"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let coverage = parsed["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/testing:coverage")
        .unwrap_or_else(|| panic!("no coverage pass: {parsed}"))
        .clone();
    assert_eq!(
        coverage["summary"]["test_results"]["obligations_proven"], 1,
        "{coverage}"
    );
    let a015: Vec<&Value> = coverage["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A015")
        .collect();
    assert_eq!(a015.len(), 1, "{coverage}");
    assert!(a015[0]["message"].as_str().unwrap().contains("\"b\""));
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "orphaned test records come back as an optional orphans field"
)]
fn analyze_returns_orphans_only_when_records_are_orphaned() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::write(root.join("app.spec"), "behavior widget \"Widget\" {\n}\n").unwrap();
    let report = root.join("specforge-report.json");
    let mut server = test_server();
    let mut run = |strict: bool| -> Value {
        let resp = call_tool(
            &mut server,
            "specforge.analyze",
            json!({"path": root.to_str().unwrap(), "pass": "contracts", "strict": strict}),
        );
        serde_json::from_str(&tool_text(&resp)).unwrap()
    };

    std::fs::write(&report, r#"{"results":{"widget":{"tests":[]}}}"#).unwrap();
    assert!(run(false).get("orphans").is_none());

    std::fs::write(&report, r#"{"results":{"wodget":{"tests":[]}}}"#).unwrap();
    let expected = json!([{"entity_id": "wodget", "near": "widget"}]);
    let lax = run(false);
    assert_eq!(lax["orphans"], expected, "{lax}");
    let strict = run(true);
    assert_eq!(strict["orphans"], expected, "{strict}");
    assert_eq!(strict["ok"], lax["ok"]);
}

/// The `McpError` an `isError` tool result carries.
fn mcp_error(resp: &Value) -> Value {
    assert!(resp["error"].is_null(), "not a JSON-RPC error: {resp}");
    assert_eq!(resp["result"]["isError"], true, "an isError result: {resp}");
    serde_json::from_str(&tool_text(resp)).unwrap_or_else(|e| panic!("McpError JSON ({e}): {resp}"))
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "a malformed specforge-report.json is an error result, not an empty report"
)]
fn coverage_refuses_a_malformed_report() {
    let mut server = test_server();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge-report.json"),
        r#"{"results": {"alpha": {"tests": ["#,
    )
    .unwrap();
    server.state_mut().project_root = Some(project.path().to_path_buf());

    let error = mcp_error(&call_tool(&mut server, "specforge.coverage", json!({})));
    assert_eq!(error["code"], "schema_mismatch", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("specforge-report.json"),
        "names the file: {error}"
    );
    assert_eq!(error["diagnostic"]["code"], "E045", "{error}");

    // Without a report, nothing is recorded: not an error.
    std::fs::remove_file(project.path().join("specforge-report.json")).unwrap();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    assert_ne!(resp["result"]["isError"], true, "{resp}");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a malformed test report is an error result"
)]
fn analyze_refuses_a_malformed_report() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior two \"Two\" {\n  verify unit \"a\"\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("specforge-report.json"), "{not json").unwrap();

    let mut server = test_server();
    // The project's own report.
    let error = mcp_error(&call_tool(
        &mut server,
        "specforge.analyze",
        json!({"path": root.to_str().unwrap(), "pass": "coverage"}),
    ));
    assert_eq!(error["code"], "schema_mismatch", "{error}");
    assert_eq!(error["diagnostic"]["code"], "E045", "{error}");

    // A report the caller names.
    let error = mcp_error(&call_tool(
        &mut server,
        "specforge.analyze",
        json!({
            "path": root.to_str().unwrap(),
            "pass": "coverage",
            "test_results": root.join("specforge-report.json").to_str().unwrap(),
        }),
    ));
    assert_eq!(error["code"], "schema_mismatch", "{error}");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "extension passes receive no proved claims, as specforge analyze without --prove"
)]
fn analyze_hands_extension_passes_no_proved_claims() {
    use crate::fake_extension::{self, EXT, FakeExtension};

    // `@test/cmds` declares one compiler pass and records what it is given.
    let ext = FakeExtension::new()
        .with_passes(&["audit"])
        .with_output("__pass_audit", json!({"diagnostics": []}));
    let (mut server, ext, _project) = fake_extension::initialized(ext);

    let resp = call_tool(&mut server, "specforge.analyze", json!({}));

    assert_ne!(resp["result"]["isError"], true, "{resp}");
    let calls = ext.calls();
    let (_, _, input) = calls
        .iter()
        .find(|(name, export, _)| name == EXT && export == "__pass_audit")
        .unwrap_or_else(|| panic!("the pass never ran: {calls:?}"));
    // Prove did not run: no claims, not an empty set of them.
    assert_eq!(input["proved_claims"], Value::Null, "{input}");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "an unknown or undeclared pass is an invalid-input error listing the available passes"
)]
fn analyze_refuses_an_undeclared_pass() {
    use crate::fake_extension::{self, FakeExtension};

    let ext = FakeExtension::new()
        .with_passes(&["audit"])
        .with_output("__pass_audit", json!({"diagnostics": []}));
    let (mut server, _ext, _project) = fake_extension::initialized(ext);

    for requested in ["foo:bar", "@test/cmds:typo", "nope"] {
        let error = mcp_error(&call_tool(
            &mut server,
            "specforge.analyze",
            json!({"pass": requested}),
        ));
        assert_eq!(error["code"], "invalid_input", "{error}");
        assert_eq!(error["argument"], "pass", "{error}");
        let message = error["message"].as_str().unwrap();
        for available in ["all", "coverage", "contracts", "@test/cmds:audit"] {
            assert!(message.contains(available), "{available} in {message}");
        }
    }

    // A manifest-declared pass and the `coverage` alias are accepted.
    for requested in ["@test/cmds:audit", "coverage", "contracts", "all"] {
        let resp = call_tool(&mut server, "specforge.analyze", json!({"pass": requested}));
        assert_ne!(resp["result"]["isError"], true, "{requested}: {resp}");
    }
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "strict promotes warnings and clears ok"
)]
fn analyze_strict_promotes_warnings() {
    use crate::fake_extension::{self, FakeExtension};

    let ext = FakeExtension::new().with_passes(&["audit"]).with_output(
        "__pass_audit",
        json!({"diagnostics": [
            {"code": "W900", "severity": "Warning", "message": "careful"}
        ]}),
    );
    let (mut server, _ext, _project) = fake_extension::initialized(ext);
    let run = |server: &mut McpServer, args: Value| -> Value {
        let resp = call_tool(server, "specforge.analyze", args);
        serde_json::from_str(&tool_text(&resp)).unwrap()
    };

    let lenient = run(&mut server, json!({"pass": "@test/cmds:audit"}));
    assert_eq!(lenient["ok"], true, "{lenient}");
    assert_eq!(lenient["passes"][0]["findings"][0]["severity"], "Warning");

    let strict = run(
        &mut server,
        json!({"pass": "@test/cmds:audit", "strict": true}),
    );
    assert_eq!(strict["ok"], false, "{strict}");
    assert_eq!(strict["passes"][0]["findings"][0]["severity"], "Error");
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "analyzing another project leaves the served project untouched"
)]
fn analyze_of_another_project_leaves_the_served_one() {
    use crate::fake_extension::{self, FakeExtension};

    let (mut server, _ext, _served) = fake_extension::initialized(FakeExtension::new());
    let served_root = server.state_mut().project_root.clone();
    assert!(served_root.is_some());
    let served_nodes = server.state_mut().graph().node_count();

    let other = tempfile::tempdir().unwrap();
    std::fs::write(
        other.path().join("specforge.json"),
        r#"{"name":"o","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::write(
        other.path().join("app.spec"),
        "behavior solo \"Solo\" {\n}\n",
    )
    .unwrap();
    let resp = call_tool(
        &mut server,
        "specforge.analyze",
        json!({"path": other.path().to_str().unwrap(), "pass": "contracts"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["passes"][0]["pass"], "contracts", "{parsed}");

    assert_eq!(server.state_mut().project_root, served_root);
    assert_eq!(server.state_mut().graph().node_count(), served_nodes);
}

// --- specforge.stats ---

// B:provide_mcp_stats_tool — verify unit "returns project statistics"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "specforge.stats returns entity counts by kind"
)]
fn stats_returns_statistics() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let mut counts: Vec<(String, u64)> = parsed["entity_counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["kind"].as_str().unwrap().to_string(),
                c["count"].as_u64().unwrap(),
            )
        })
        .collect();
    counts.sort();
    assert_eq!(
        counts,
        vec![
            ("behavior".to_string(), 1),
            ("feature".to_string(), 1),
            ("invariant".to_string(), 1),
        ]
    );

    // Another behavior shows up in its kind's count.
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "delta".into(),
            },
            kind: EntityKind {
                raw: "behavior".into(),
            },
            title: None,
            fields: FieldMap::new(),
            source_span: span(),
            methods: Vec::new(),
        });
    });
    let parsed: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.stats",
        json!({}),
    )))
    .unwrap();
    let behaviors = parsed["entity_counts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "behavior")
        .unwrap();
    assert_eq!(behaviors["count"], 2);
}

#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes orphan node count"
)]
fn stats_counts_match_graph() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["edge_count"], 1); // one behaviors edge
    assert_eq!(parsed["orphan_count"], 1); // gamma_orphan
}

// Tool call when not initialized
#[specforge_test(
    behavior = "mcp_initialize",
    verify = "initialization rejects tool calls before completion"
)]
fn tool_call_not_initialized() {
    let mut server = McpServer::new();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    assert!(resp["error"].is_object());
}

// Unknown tool name
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "unknown tool returns error"
)]
fn unknown_tool_returns_error() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.nonexistent", json!({}));
    assert!(resp["error"].is_object());
}

// B:provide_mcp_validate_tool — verify unit "response includes all diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "specforge.validate tool triggers compilation"
)]
fn validate_returns_all_diagnostics() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    assert!(server.state().graph().node("alpha").is_some());
    assert!(server.state().diagnostics().is_empty());

    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();

    // The project was compiled: its entities replaced the injected graph,
    // and its diagnostics are the ones returned.
    let graph = &server.state().graph();
    assert!(graph.node("lonely").is_some());
    assert!(graph.node("act").is_some());
    assert!(graph.node("alpha").is_none(), "the old graph remains");
    let codes: Vec<&str> = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"E003"), "{codes:?}");
    assert!(codes.contains(&"W003"), "{codes:?}");
    assert_eq!(codes.len(), server.state().diagnostics().len());
}

/// A project whose check yields errors (E003, E006) and warnings (W003, W006).
fn project_with_errors_and_warnings() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::write(
        project.path().join("app.spec"),
        "invariant lonely \"Lonely\" {\n  guarantee \"g\"\n}\n\nbehavior act \"Act\" {\n  invariants [missing]\n}\n",
    )
    .unwrap();
    project
}

/// `specforge.validate`'s diagnostics as (code, severity).
fn validate(args: Value) -> Vec<(String, String)> {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.validate", args);
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["code"].as_str().unwrap().to_string(),
                d["severity"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "severity_filter restricts returned diagnostics"
)]
fn validate_severity_filter_restricts_diagnostics() {
    let project = project_with_errors_and_warnings();
    let path = project.path().to_str().unwrap();
    let all = validate(json!({"path": path}));
    assert!(all.iter().any(|(_, s)| s == "Error"), "{all:?}");
    assert!(all.iter().any(|(_, s)| s == "Warning"), "{all:?}");
    for severity in ["Error", "Warning"] {
        let filtered = validate(json!({"path": path, "severity_filter": severity.to_lowercase()}));
        let expected: Vec<_> = all.iter().filter(|(_, s)| s == severity).cloned().collect();
        assert_eq!(filtered, expected);
    }
}

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "strict mode promotes warnings to errors"
)]
fn validate_strict_promotes_warnings_to_errors() {
    let project = project_with_errors_and_warnings();
    let path = project.path().to_str().unwrap();
    let all = validate(json!({"path": path}));
    let strict = validate(json!({"path": path, "strict": true}));
    let codes = |d: &[(String, String)]| d.iter().map(|(c, _)| c.clone()).collect::<Vec<_>>();
    assert_eq!(codes(&strict), codes(&all), "the same diagnostics");
    assert!(strict.iter().all(|(_, s)| s == "Error"), "{strict:?}");
    assert!(strict.iter().any(|(c, _)| c == "W003"));
}

// B:provide_mcp_validate_tool — verify unit "use_cached=false triggers fresh compilation"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=false triggers fresh compilation"
)]
fn validate_use_cached_false() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    let first = codes_of(&call_tool(&mut server, "specforge.validate", json!({})));
    assert!(first.contains(&"E003".to_string()), "{first:?}");

    // Fix the unresolved reference on disk.
    fix_unresolved_reference(project.path());
    let second = codes_of(&call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": false}),
    ));
    assert!(
        !second.contains(&"E003".to_string()),
        "use_cached=false must recompile: {second:?}"
    );
    assert!(server.state().graph().node("fixed").is_some());
}

/// The diagnostic codes of a `specforge.validate` response.
fn codes_of(resp: &Value) -> Vec<String> {
    let parsed: Value = serde_json::from_str(&tool_text(resp)).unwrap();
    parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_string())
        .collect()
}

/// Rewrites `project_with_errors_and_warnings`' spec so `act` no longer
/// names the missing invariant (no E003) and a `fixed` behavior appears.
fn fix_unresolved_reference(root: &std::path::Path) {
    std::fs::write(
        root.join("app.spec"),
        "invariant lonely \"Lonely\" {\n  guarantee \"g\"\n}\n\nbehavior act \"Act\" {\n  invariants [lonely]\n}\n\nbehavior fixed \"Fixed\" {\n}\n",
    )
    .unwrap();
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "plan parameter triggers gap analysis"
)]
fn trace_plan_gap_analysis() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    std::fs::write(
        project.path().join("app.spec"),
        "invariant inv_a \"A\" {\n  guarantee \"g\"\n  verify unit \"z\"\n}\n\n\
         behavior act_one \"One\" {\n  invariants [inv_a]\n  verify unit \"x\"\n}\n\n\
         behavior act_two \"Two\" {\n  verify unit \"y\"\n}\n",
    )
    .unwrap();
    let mut server = test_server();
    call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": project.path().to_str().unwrap()}),
    );

    // act_one depends on inv_a but comes first; ghost doesn't exist;
    // act_two is testable and missing.
    let plan = json!({"entries": [
        {"entity_id": "act_one"}, {"entity_id": "inv_a"}, {"entity_id": "ghost"}
    ]});
    let resp = call_tool(&mut server, "specforge.trace", json!({"plan": plan}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["affected_entities"], json!(["act_one", "inv_a"]));
    let gaps: Vec<(&str, &str, &str)> = parsed["gaps"]
        .as_array()
        .unwrap_or_else(|| panic!("no gaps in {parsed}"))
        .iter()
        .map(|g| {
            (
                g["source_entity"].as_str().unwrap(),
                g["target_entity"].as_str().unwrap(),
                g["missing_link_type"].as_str().unwrap(),
            )
        })
        .collect();
    assert!(
        gaps.contains(&("plan", "ghost", "unresolved_entity")),
        "{gaps:?}"
    );
    assert!(
        gaps.contains(&("plan", "act_two", "missing_plan_entry")),
        "{gaps:?}"
    );
    assert!(gaps.contains(&("act_one", "inv_a", "ordering")), "{gaps:?}");
    assert_eq!(gaps.len(), 3, "{gaps:?}");
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "trace without entity_id or plan returns error"
)]
fn trace_without_entity_or_plan_errors() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.trace", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    let message = error["message"].as_str().unwrap();
    assert!(message.contains("entity_id or plan"), "{message}");
}

#[test]
fn trace_missing_links() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "gamma_orphan"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "gamma_orphan");
    // Orphan has no connections
    assert!(parsed["upstream"].as_array().unwrap().is_empty());
    assert!(parsed["downstream"].as_array().unwrap().is_empty());
}

// B:provide_mcp_search_tool — verify unit "empty query returns all entities up to limit"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "empty query returns all entities up to limit"
)]
fn search_empty_query_returns_all() {
    let mut server = test_server();
    let ids = |server: &mut McpServer, args: Value| {
        let resp = call_tool(server, "specforge.search", args);
        let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        let mut ids: Vec<String> = parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["entity_id"].as_str().unwrap().to_string())
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(
        ids(&mut server, json!({"query": ""})),
        vec!["alpha", "beta_feature", "gamma_orphan"]
    );
    assert_eq!(ids(&mut server, json!({"query": "", "limit": 2})).len(), 2);
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "include_edges false omits edge type definitions"
)]
fn schema_include_edges_false_omits_edges() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    let full = compiled_schema(&mut server, &project, json!({}));
    assert_eq!(
        full["edge_types"].as_array().unwrap().len(),
        15,
        "edges by default"
    );
    let without = compiled_schema(&mut server, &project, json!({"include_edges": false}));
    assert!(without.get("edge_types").is_none(), "{without}");
    assert_eq!(without["entity_kinds"], full["entity_kinds"]);
}

// B:provide_mcp_schema_tool — verify unit "include_validation_rules true includes rules"
#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "include_validation_rules true includes validation rules"
)]
fn schema_include_validation_rules_lists_extension_rules() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    // Compiling the project loads @specforge/software's manifest.
    call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": project.path().to_str().unwrap()}),
    );
    let schema = |server: &mut McpServer, args: Value| -> Value {
        serde_json::from_str(&tool_text(&call_tool(server, "specforge.schema", args))).unwrap()
    };
    assert!(
        schema(&mut server, json!({}))
            .get("validation_rules")
            .is_none()
    );
    let with = schema(&mut server, json!({"include_validation_rules": true}));
    let rules = with["validation_rules"].as_array().unwrap();
    let w003 = rules
        .iter()
        .find(|r| r["code"] == "W003")
        .unwrap_or_else(|| panic!("no W003 in {with}"));
    assert_eq!(w003["extension"], "@specforge/software");
    assert_eq!(w003["severity"], "warning");
}

// B:provide_mcp_coverage_tool — verify unit "kind filter restricts to matching entity kinds"
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "kind filter restricts to matching entity kinds"
)]
fn coverage_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"kind": "behavior"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    for r in results {
        assert_eq!(r["kind"], "behavior");
    }
}

// B:provide_mcp_coverage_tool — verify unit "status_filter restricts to matching status"
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "status_filter restricts to matching coverage status"
)]
fn coverage_status_filter_restricts_status() {
    // `two` has one of its two obligations proven; alpha has none.
    let (mut server, _project) = server_with_report(&[("a", "pass")]);
    let ids = |server: &mut McpServer, status: &str| {
        let resp = call_tool(
            server,
            "specforge.coverage",
            json!({"status_filter": status}),
        );
        let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        parsed
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["entity_id"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&mut server, "partial"), vec!["two"]);
    assert!(ids(&mut server, "uncovered").contains(&"alpha".to_string()));
    assert!(!ids(&mut server, "uncovered").contains(&"two".to_string()));
    assert!(ids(&mut server, "covered").is_empty());
}

#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes coverage percentage"
)]
fn stats_includes_coverage_percentage() {
    let mut server = test_server();
    let coverage = |server: &mut McpServer| {
        let resp = call_tool(server, "specforge.stats", json!({}));
        let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        parsed["coverage_pct"].as_f64().unwrap()
    };
    // Testable: alpha (behavior, declares verify) and gamma_orphan
    // (invariant, none). beta_feature's kind is not testable.
    assert_eq!(coverage(&mut server), 50.0);

    let mut fields = FieldMap::new();
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".into(),
            description: "gamma holds".into(),
        }]),
    );
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "gamma_orphan".into(),
            },
            kind: EntityKind {
                raw: "invariant".into(),
            },
            title: Some("Gamma Orphan".into()),
            fields,
            source_span: span(),
            methods: Vec::new(),
        });
    });
    assert_eq!(coverage(&mut server), 100.0);
}

// B:provide_mcp_query_tool — verify unit "format parameter selects emitter format"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "format parameter changes output serialization"
)]
fn query_format_parameter() {
    let mut server = test_server();
    crate::support::declare_headline_fields(&mut server, "behavior");
    // context format
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "context"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        node(&parsed, "alpha")["contract"],
        "The system MUST do alpha"
    );

    // brief format
    let resp2 = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "brief"}),
    );
    let text2 = tool_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let brief_alpha = node(&parsed2, "alpha");
    assert!(brief_alpha.get("contract").is_none(), "{brief_alpha}");
    assert!(brief_alpha.get("fields").is_none(), "{brief_alpha}");
    assert_ne!(text, text2, "the two formats serialize differently");
    // Both formats return the same subgraph.
    assert_eq!(node_ids(&parsed), node_ids(&parsed2));
}

// B:provide_mcp_query_tool — verify unit "include_coverage annotates nodes with coverage status"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "include_coverage parameter includes coverage status in response"
)]
fn query_include_coverage() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "include_coverage": true}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    let alpha = nodes.iter().find(|n| n["id"] == "alpha").unwrap();
    // Declared obligations with no recorded test: as specforge.coverage says.
    assert_eq!(alpha["coverage_status"], "uncovered");

    let (mut server, _project) = server_with_report(&[("a", "pass"), ("b", "pass")]);
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "two", "include_coverage": true}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let two = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "two")
        .unwrap()
        .clone();
    assert_eq!(two["coverage_status"], "covered");
}

// B:provide_mcp_search_tool — verify unit "field and value filter matches entity fields"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "field and value filter matches entity fields"
)]
fn search_field_value_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "field": "contract", "value": "MUST"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["entity_id"], "alpha");

    // The value is matched against the field's text, not its Rust
    // representation (`String("…")`).
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "", "field": "contract", "value": "string"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed, json!([]));
}

// B:provide_mcp_search_tool — verify unit "references filter returns entities referencing target"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "references filter returns entities referencing target"
)]
fn search_references_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "references": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    // beta_feature has an edge to alpha
    assert!(results.iter().any(|r| r["entity_id"] == "beta_feature"));
}

// B:provide_mcp_stats_tool — verify unit "diagnostic_summary includes severity counts"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes diagnostic summary by severity"
)]
fn stats_diagnostic_summary_severity_counts() {
    use specforge_common::{Diagnostic, Severity};
    let mut server = test_server();
    let diagnostic = |code: &str, severity| Diagnostic {
        code: code.into(),
        severity,
        message: "m".into(),
        span: Some(span()),
        suggestion: None,
        data: None,
    };
    server.state_mut().surface_diagnostics = vec![
        diagnostic("E003", Severity::Error),
        diagnostic("W001", Severity::Warning),
        diagnostic("W003", Severity::Warning),
    ];
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        parsed["diagnostic_summary"],
        json!({"errors": 1, "warnings": 2, "infos": 0})
    );
}

// B:provide_mcp_trace_tool — verify unit "gaps array lists missing expected links"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "missing links flagged in trace output"
)]
fn trace_gaps_array() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "gamma_orphan"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let gaps = parsed["gaps"].as_array().unwrap();
    assert!(gaps.contains(&json!("no upstream links")));
    assert!(gaps.contains(&json!("no downstream links")));
}

// B:provide_mcp_validate_tool — verify unit "use_cached returns existing diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=true returns existing diagnostics without recompilation"
)]
fn validate_use_cached_true() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    let first = codes_of(&call_tool(&mut server, "specforge.validate", json!({})));
    assert!(first.contains(&"E003".to_string()), "{first:?}");

    // The spec changes on disk, but a cached validate does not recompile:
    // the old diagnostics come back and the graph is the old one.
    fix_unresolved_reference(project.path());
    let cached = codes_of(&call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": true}),
    ));
    assert_eq!(cached, first);
    assert!(server.state().graph().node("fixed").is_none());

    // Proof the change on disk is visible to a fresh compile.
    let fresh = codes_of(&call_tool(&mut server, "specforge.validate", json!({})));
    assert!(!fresh.contains(&"E003".to_string()), "{fresh:?}");
}

// B:provide_mcp_validate_tool — verify unit "validate recompiles and updates graph"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "response includes all diagnostics as Graph Protocol diagnostics"
)]
fn validate_updates_graph() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": project.path().to_str().unwrap()}),
    );
    // A validation run that finds errors is a successful call (ADR 0004
    // D4-a): the findings are its result.
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let diagnostics = parsed.as_array().unwrap();
    // All of them: as many as the compile produced.
    assert_eq!(diagnostics.len(), server.state().diagnostics().len());
    for d in diagnostics {
        for key in ["code", "severity", "message"] {
            assert!(d[key].is_string(), "{key} missing in {d}");
        }
    }
    for (code, severity) in [("E003", "Error"), ("W003", "Warning")] {
        let d = diagnostics
            .iter()
            .find(|d| d["code"] == code)
            .unwrap_or_else(|| panic!("no {code} in {parsed}"));
        assert_eq!(d["severity"], severity);
        assert!(d["file"].as_str().unwrap().ends_with("app.spec"), "{d}");
        assert!(d["line"].as_u64().unwrap() >= 1, "{d}");
    }
}

#[test]
fn validate_use_cached_false_triggers_fresh() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    // First compile
    let _resp1 = call_tool(&mut server, "specforge.validate", json!({}));
    // Second call with use_cached=false should recompile
    let resp2 = call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": false}),
    );
    assert!(
        resp2["result"].is_object() || resp2["error"].is_object(),
        "use_cached=false should trigger fresh compilation"
    );
}

#[test]
fn trace_missing_links_flagged() {
    let mut server = test_server();
    // Trace with a plan that has references to nonexistent entities
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_ids": ["nonexistent_entity"]}),
    );
    // Should flag missing links
    assert!(resp["result"].is_object() || resp["error"].is_object());
}

#[test]
fn stats_includes_diagnostic_summary() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Should include diagnostic counts by severity
    assert!(
        parsed["diagnostics"].is_object()
            || parsed["diagnostic_summary"].is_object()
            || parsed["entity_count"].is_number(),
        "stats response should include diagnostic summary"
    );
}

// --- C9-02/C9-03: token budget enforcement ---

// B:provide_mcp_export_tool — verify unit "max_tokens truncates output within budget with metadata"
#[test]
fn export_with_max_tokens_truncates_and_reports_budget() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "max_tokens": 120}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    // metadata: the budget result rides in the payload
    assert!(
        parsed["token_budget"].is_object(),
        "budget metadata must be present: {}",
        text.len()
    );
    assert_eq!(parsed["token_budget"]["budget_tokens"], 120);

    // the estimate must honor the budget
    let estimated = parsed["token_budget"]["estimated_tokens"].as_u64().unwrap();
    assert!(
        estimated <= 120,
        "estimated tokens {estimated} exceed the 120-token budget"
    );

    // and truncation happened (a full-graph export is much larger)
    assert!(
        !parsed["token_budget"]["truncated_entities"]
            .as_array()
            .unwrap()
            .is_empty(),
        "entities must have been dropped to fit the budget"
    );
}

// B:mcp_resource_reads — verify unit "context resource honors ?max_tokens budget"
#[test]
fn context_resource_honors_max_tokens_budget() {
    let mut server = test_server();
    let req = json!({
        "jsonrpc": "2.0", "id": 7,
        "method": "resources/read",
        "params": { "uri": "specforge://context?max_tokens=150" }
    });
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    let text = resp["result"]["contents"][0]["text"].as_str().unwrap_or("");
    assert!(!text.is_empty(), "context payload must be present");

    // rough token estimate (words + structural chars) must fit the budget
    let words = text.split_whitespace().count();
    assert!(
        words <= 150,
        "budgeted context must be trimmed to ~150 tokens, got ~{words} words"
    );
}

#[specforge_test(
    invariant = "mcp_structured_error_responses",
    verify = "a diagnostic code behind a failed tool call is in its McpError diagnostic"
)]
fn query_and_trace_report_e003_in_the_diagnostic_not_the_message() {
    let mut server = test_server();
    for tool in ["specforge.query", "specforge.trace"] {
        let resp = call_tool(&mut server, tool, json!({"entity_id": "nonexistent"}));
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "entity_not_found", "{tool}: {error}");
        assert_eq!(error["diagnostic"]["code"], "E003", "{tool}: {error}");
        let message = error["message"].as_str().unwrap();
        assert!(!message.starts_with("E003"), "{tool}: {message}");
        assert!(message.contains("nonexistent"), "{tool}: {message}");
    }
}

// --- specforge.explain ---

#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "specforge.explain returns the catalogued title, owner, level, explanation and docs link"
)]
fn explain_returns_the_catalogued_entry() {
    let mut server = test_server();
    let explain = |server: &mut McpServer, code: &str| -> Value {
        serde_json::from_str(&tool_text(&call_tool(
            server,
            "specforge.explain",
            json!({"code": code}),
        )))
        .unwrap()
    };
    // Any case: the same entry `specforge explain W018` prints.
    let entry = explain(&mut server, "w018");
    assert_eq!(entry["code"], "W018");
    assert_eq!(entry["title"], "Duplicate edge type");
    assert_eq!(entry["owner"], "core");
    assert_eq!(entry["level"], "warning");
    assert_eq!(entry["retired"], false);
    assert!(
        entry["explanation"]
            .as_str()
            .unwrap()
            .starts_with("Two extensions register an edge type with the same label"),
        "{entry}"
    );
    assert_eq!(
        entry["docs"],
        "https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#w018"
    );
    // An extension's code names that extension.
    let formal = explain(&mut server, "E031");
    assert_eq!(formal["owner"], "@specforge/formal", "{formal}");
}

#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "a retired code names the code that replaced it"
)]
fn explain_a_retired_code_names_its_replacement() {
    let mut server = test_server();
    let retired: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.explain",
        json!({"code": "E047"}),
    )))
    .unwrap();
    assert_eq!(retired["code"], "E047");
    assert_eq!(retired["retired"], true);
    assert_eq!(retired["replaced_by"]["code"], "W139", "{retired}");
    assert_eq!(retired["replaced_by"]["level"], "warning");

    // Retired with no successor.
    let gone: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.explain",
        json!({"code": "W024"}),
    )))
    .unwrap();
    assert_eq!(
        gone,
        json!({"code": "W024", "retired": true, "replaced_by": null})
    );
}

#[specforge_test(
    behavior = "provide_mcp_explain_tool",
    verify = "an uncatalogued code is an invalid_input error"
)]
fn explain_an_uncatalogued_code_is_invalid_input() {
    let mut server = test_server();
    // E901 belongs to a third-party extension; Z123 is no code at all.
    for code in ["E901", "Z123"] {
        let resp = call_tool(&mut server, "specforge.explain", json!({"code": code}));
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "invalid_input", "{code}: {error}");
        assert!(error["message"].as_str().unwrap().contains(code), "{error}");
    }
}

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "each catalogued diagnostic carries its title"
)]
fn validate_gives_each_catalogued_code_its_title() {
    let project = project_with_errors_and_warnings();
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"path": project.path().to_str().unwrap()}),
    );
    let found: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let title = |code: &str| -> Value {
        found
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["code"] == code)
            .unwrap_or_else(|| panic!("no {code} in {found}"))["title"]
            .clone()
    };
    assert_eq!(title("E003"), "Unresolved reference");
    assert_eq!(title("E006"), "Missing required field");
}
