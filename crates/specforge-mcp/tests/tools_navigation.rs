use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
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
        source_span: span(),
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
            file: "test.spec".into(),
            start_line: 10,
            start_col: 0,
            end_line: 15,
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

    server
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

/// Adds a node with no fields at `file`:`line`:`col`.
fn add_node_at(server: &mut McpServer, id: &str, file: &str, line: usize, col: usize) {
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId { raw: id.into() },
            kind: EntityKind {
                raw: "behavior".into(),
            },
            title: None,
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: file.into(),
                start_line: line,
                start_col: col,
                end_line: line + 2,
                end_col: 0,
            },
            methods: Vec::new(),
        });
    });
}

/// The outline of `file`, as entity ids in the order returned.
fn outline_ids(server: &mut McpServer, file: &str) -> Vec<String> {
    let resp = call_tool(server, "specforge.outline", json!({"file": file}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed
        .as_array()
        .unwrap_or_else(|| panic!("no outline in {resp}"))
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap().to_string())
        .collect()
}

/// A server whose graph holds order.spec's entities out of line order:
/// `late` (line 20) is added before `early` (line 5) and `middle` (line 12).
fn server_with_unordered_file() -> McpServer {
    let mut server = test_server();
    add_node_at(&mut server, "late", "order.spec", 20, 0);
    add_node_at(&mut server, "early", "order.spec", 5, 0);
    add_node_at(&mut server, "middle", "order.spec", 12, 0);
    server
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// --- specforge.inspect ---

/// The statement inspect reports is the field the extension declares
/// headline and normative, not whatever field is named `contract`.
#[test]
fn inspect_reports_no_contract_its_kind_does_not_declare() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert!(parsed["contract"].is_null(), "{parsed}");
    assert!(parsed["fields"]["contract"].is_string(), "{parsed}");
}

// B:provide_mcp_inspect_tool — verify unit "returns entity details"
#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "specforge.inspect returns full entity details"
)]
fn inspect_returns_details() {
    let mut server = test_server();
    crate::support::declare_headline_fields(&mut server, "behavior");
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert_eq!(parsed["kind"], "behavior");
    assert!(parsed["source_span"].is_object());
    assert!(parsed["contract"].is_string());
    assert!(parsed["verify_declarations"].is_array());
}

// B:provide_mcp_inspect_tool — verify unit "includes reference count"
#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "response includes references and verify declarations"
)]
fn inspect_includes_reference_count() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // beta -> alpha is alpha's only edge.
    assert_eq!(parsed["references"], json!(["beta"]));
    assert_eq!(parsed["reference_count"], 1);
    assert_eq!(parsed["verify_declarations"], json!(["unit test alpha"]));
}

// B:provide_mcp_inspect_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "non-existent entity returns error response"
)]
fn inspect_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "nonexistent"}),
    );
    // C9-00/C9-12: entity-not-found is a tool execution error — the tool ran
    // and the domain state did not match — so it surfaces as an isError
    // result, not a -32602 protocol error.
    assert!(
        resp["result"]["isError"] == true,
        "domain failure must be an isError tool result"
    );
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "response includes every field, like an invariant's guarantee"
)]
fn inspect_returns_every_field() {
    let mut server = test_server();
    let mut fields = FieldMap::new();
    fields.push(
        "guarantee".into(),
        FieldValue::String("Ids MUST be unique".into()),
    );
    fields.push("risk".into(), FieldValue::Identifier("medium".into()));
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "unique_ids".into(),
            },
            kind: EntityKind {
                raw: "invariant".into(),
            },
            title: Some("Unique ids".into()),
            fields,
            source_span: span(),
            methods: Vec::new(),
        });
    });
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "unique_ids"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed["fields"]["guarantee"], "Ids MUST be unique");
    assert_eq!(parsed["fields"]["risk"], "medium");
    assert!(parsed["contract"].is_null(), "an invariant has no contract");
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "coverage status matches specforge.coverage obligation by obligation"
)]
fn inspect_coverage_matches_the_coverage_tool() {
    let mut server = test_server();
    let statuses = |server: &mut McpServer| {
        let resp = call_tool(server, "specforge.inspect", json!({"entity_id": "alpha"}));
        let inspect: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        let resp = call_tool(server, "specforge.coverage", json!({"entity_id": "alpha"}));
        let coverage: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        (
            inspect["coverage_status"].as_str().unwrap().to_string(),
            coverage[0]["status"].as_str().unwrap().to_string(),
        )
    };
    let project = tempfile::tempdir().unwrap();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    let report = |tests: &str| {
        std::fs::write(
            project.path().join("specforge-report.json"),
            format!(r#"{{"results":{{"alpha":{{"tests":[{tests}]}}}}}}"#),
        )
        .unwrap();
    };

    // A passing test that names no obligation proves none of them.
    report(r#"{"name":"t","status":"pass"}"#);
    let (inspect, coverage) = statuses(&mut server);
    assert_eq!(inspect, "uncovered");
    assert_eq!(inspect, coverage);

    report(r#"{"name":"t","status":"pass","verify":"test alpha"}"#);
    let (inspect, coverage) = statuses(&mut server);
    assert_eq!(inspect, "covered");
    assert_eq!(inspect, coverage);
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "diagnostics are the entity's own, not those of an entity whose ID contains it"
)]
fn inspect_diagnostics_are_the_entitys_own() {
    let mut server = test_server();
    let at = |start_line, end_line| SourceSpan {
        file: "test.spec".into(),
        start_line,
        start_col: 1,
        end_line,
        end_col: 2,
    };
    let node = |id: &str, span: SourceSpan| Node {
        id: EntityId { raw: id.into() },
        kind: EntityKind {
            raw: "invariant".into(),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: span,
        methods: Vec::new(),
    };
    let state = server.state_mut();
    state.edit_graph(|graph| {
        graph.add_node(node("task", at(20, 22)));
    });
    state.edit_graph(|graph| {
        graph.add_node(node("task_id_uniqueness", at(30, 34)));
    });
    let diagnostic = |code: &str, message: &str, span| specforge_common::Diagnostic {
        code: code.into(),
        severity: specforge_common::Severity::Warning,
        message: message.into(),
        span,
        suggestion: None,
    };
    state.surface_diagnostics = vec![
        diagnostic(
            "W003",
            "invariant 'task_id_uniqueness' is not enforced",
            Some(at(30, 34)),
        ),
        diagnostic("W100", "field inside task", Some(at(21, 21))),
        diagnostic("W101", "invariant 'task' is spanless", None),
        diagnostic("W102", "invariant 'task_id_uniqueness' is spanless", None),
    ];
    let codes = |server: &mut McpServer, id: &str| {
        let resp = call_tool(server, "specforge.inspect", json!({"entity_id": id}));
        let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
        parsed["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["code"].as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(&mut server, "task"), vec!["W100", "W101"]);
    assert_eq!(
        codes(&mut server, "task_id_uniqueness"),
        vec!["W003", "W102"]
    );
}

// --- specforge.find_definition ---

// B:provide_mcp_find_definition_tool — verify unit "returns source location"
#[specforge_test(
    behavior = "provide_mcp_find_definition_tool",
    verify = "specforge.find_definition returns file, line, and column"
)]
fn find_definition_returns_location() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert_eq!(parsed["file_path"], "test.spec");
    assert_eq!(parsed["line"], 1);
    assert_eq!(parsed["column"], 0);

    // An entity declared mid-line reports its own line and column.
    add_node_at(&mut server, "indented", "nested.spec", 7, 4);
    let resp = call_tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "indented"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(
        parsed,
        json!({"entity_id": "indented", "file_path": "nested.spec", "line": 7, "column": 4})
    );
}

// B:provide_mcp_find_definition_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_find_definition_tool",
    verify = "non-existent entity returns error response"
)]
fn find_definition_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "nonexistent"}),
    );
    // C9-00/C9-12: entity-not-found is a tool execution error — the tool ran
    // and the domain state did not match — so it surfaces as an isError
    // result, not a -32602 protocol error.
    assert!(
        resp["result"]["isError"] == true,
        "domain failure must be an isError tool result"
    );
}

// --- specforge.find_references ---

// B:provide_mcp_find_references_tool — verify unit "returns referencing entities"
#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "specforge.find_references returns all reference locations"
)]
fn find_references_returns_refs() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    let locations = parsed["locations"].as_array().unwrap();
    assert!(!locations.is_empty());
    assert_eq!(locations[0]["referencing_entity_id"], "beta");
}

// B:provide_mcp_find_references_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "non-existent entity returns error response"
)]
fn find_references_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "nonexistent"}),
    );
    // C9-00/C9-12: entity-not-found is a tool execution error — the tool ran
    // and the domain state did not match — so it surfaces as an isError
    // result, not a -32602 protocol error.
    assert!(
        resp["result"]["isError"] == true,
        "domain failure must be an isError tool result"
    );
}

// --- specforge.outline ---

// B:provide_mcp_outline_tool — verify unit "returns entities in file"
#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "specforge.outline returns all entities defined in file"
)]
fn outline_returns_entities_in_file() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "test.spec"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entries = parsed.as_array().unwrap();
    for entry in entries {
        assert_eq!(entry["range"]["file"], "test.spec");
    }
    // Both of test.spec's entities, and none from another file.
    add_node_at(&mut server, "elsewhere", "other.spec", 3, 0);
    assert_eq!(outline_ids(&mut server, "test.spec"), vec!["alpha", "beta"]);
    assert_eq!(outline_ids(&mut server, "other.spec"), vec!["elsewhere"]);
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "non-existent file returns error response"
)]
fn outline_of_a_missing_file_is_an_error() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "nonexistent.spec"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "file_not_found", "{error}");
    let message = error["message"].as_str().unwrap_or_default();
    assert!(message.contains("nonexistent.spec"), "{resp}");
}

#[test]
fn outline_of_an_existing_file_without_entities_is_empty() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("empty.spec"), "// nothing yet\n").unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());

    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "empty.spec"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(parsed, json!([]));
}

// B:provide_mcp_outline_tool — verify unit "sorted by line number"
#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "sorted by line number"
)]
fn outline_sorted_by_line() {
    let mut server = server_with_unordered_file();
    assert_eq!(
        outline_ids(&mut server, "order.spec"),
        vec!["early", "middle", "late"]
    );
}

// --- specforge.suggest_fixes ---

// B:provide_mcp_suggest_fixes_tool — verify unit "returns suggestions from diagnostics"
#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "specforge.suggest_fixes returns applicable fix suggestions"
)]
fn suggest_fixes_returns_suggestions() {
    let mut server = test_server();
    // Add a diagnostic with suggestion
    server
        .state_mut()
        .surface_diagnostics
        .push(specforge_common::Diagnostic {
            code: "W001".into(),
            severity: specforge_common::Severity::Warning,
            message: "alpha has no tests field".into(),
            span: Some(span()),
            suggestion: Some("Add a tests field".into()),
        });

    let resp = call_tool(
        &mut server,
        "specforge.suggest_fixes",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let suggestions = parsed.as_array().unwrap();
    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0]["kind"], "quickfix");
}

/// `test_server` with one fixable diagnostic inside alpha's span and one,
/// spanless, that names beta.
fn server_with_fixable_diagnostics() -> McpServer {
    use specforge_common::{Diagnostic, Severity};
    let mut server = test_server();
    server.state_mut().surface_diagnostics.push(Diagnostic {
        code: "V001".into(),
        severity: Severity::Error,
        message: "alpha is missing a field".into(),
        span: Some(span()),
        suggestion: Some("fix alpha".into()),
    });
    server.state_mut().surface_diagnostics.push(Diagnostic {
        code: "W001".into(),
        severity: Severity::Warning,
        message: "feature 'beta' has no owner".into(),
        span: None,
        suggestion: Some("fix beta".into()),
    });
    server
}

fn fix_titles(server: &mut McpServer, args: Value) -> Vec<String> {
    let resp = call_tool(server, "specforge.suggest_fixes", args);
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["title"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "clean entity with no diagnostics returns empty list"
)]
fn suggest_fixes_for_a_clean_entity_is_empty() {
    use specforge_common::{Diagnostic, Severity};
    let mut server = test_server();
    server.state_mut().surface_diagnostics.push(Diagnostic {
        code: "V001".into(),
        severity: Severity::Error,
        message: "alpha is missing a field".into(),
        span: Some(span()),
        suggestion: Some("fix alpha".into()),
    });
    // About another entity whose id merely contains beta's.
    server.state_mut().surface_diagnostics.push(Diagnostic {
        code: "W001".into(),
        severity: Severity::Warning,
        message: "feature 'beta_two' has no owner".into(),
        span: None,
        suggestion: Some("fix beta_two".into()),
    });

    assert!(fix_titles(&mut server, json!({"entity_id": "beta"})).is_empty());
    assert_eq!(
        fix_titles(&mut server, json!({"entity_id": "alpha"})),
        ["fix alpha"]
    );
}

#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "diagnostic_code filter restricts to matching diagnostics"
)]
fn suggest_fixes_diagnostic_code_filter() {
    let mut server = server_with_fixable_diagnostics();

    assert_eq!(
        fix_titles(&mut server, json!({})),
        ["fix alpha", "fix beta"]
    );
    assert_eq!(
        fix_titles(&mut server, json!({"diagnostic_code": "W001"})),
        ["fix beta"]
    );
}

#[test]
fn suggest_fixes_entity_and_file_filters() {
    let mut server = server_with_fixable_diagnostics();

    assert_eq!(
        fix_titles(&mut server, json!({"entity_id": "beta"})),
        ["fix beta"]
    );
    assert_eq!(
        fix_titles(&mut server, json!({"file_path": "test.spec"})),
        ["fix alpha"]
    );
    assert!(fix_titles(&mut server, json!({"file_path": "other.spec"})).is_empty());
    let unknown = call_tool(
        &mut server,
        "specforge.suggest_fixes",
        json!({"entity_id": "no_such_entity"}),
    );
    let error = crate::tool_errors::mcp_error(&unknown);
    assert_eq!(error["code"], "entity_not_found", "{error}");
    assert_eq!(error["entity_id"], "no_such_entity", "{error}");
}

// B:provide_mcp_find_references_tool — verify unit "entity with no references returns empty list"
#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "entity with no references returns empty list"
)]
fn find_references_empty_list() {
    let mut server = test_server();
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "orphan_node".into(),
            },
            kind: EntityKind {
                raw: "behavior".into(),
            },
            title: Some("Orphan".into()),
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: "orphan.spec".into(),
                start_line: 1,
                start_col: 0,
                end_line: 3,
                end_col: 0,
            },
            methods: Vec::new(),
        });
    });
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "orphan_node"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["locations"].as_array().unwrap().is_empty());
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "nested entries included for complex entities"
)]
fn outline_nests_an_entitys_methods() {
    let mut server = test_server();
    let method_span = SourceSpan {
        file: "store.spec".into(),
        start_line: 3,
        start_col: 2,
        end_line: 3,
        end_col: 40,
    };
    server.state_mut().edit_graph(|graph| {
        graph.add_node(Node {
            id: EntityId {
                raw: "store".into(),
            },
            kind: EntityKind { raw: "port".into() },
            title: Some("Store".into()),
            fields: FieldMap::new(),
            source_span: SourceSpan {
                file: "store.spec".into(),
                start_line: 1,
                start_col: 0,
                end_line: 5,
                end_col: 1,
            },
            methods: vec![specforge_parser::MethodDecl {
                name: "load".into(),
                params: vec![specforge_parser::Parameter {
                    name: "path".into(),
                    ty: "Path".into(),
                    optional: false,
                    annotations: Vec::new(),
                }],
                returns: Some("Store".into()),
                span: method_span,
            }],
        });
    });

    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "store.spec"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();

    let children = parsed[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 1, "{parsed}");
    assert_eq!(children[0]["entity_id"], "store.load");
    assert_eq!(children[0]["kind"], "method");
    assert_eq!(children[0]["title"], "load(path: Path) -> Store");
    assert_eq!(children[0]["range"]["start_line"], 3);
    // An entity without members has no children key.
    let flat = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "test.spec"}),
    );
    let flat: Value = serde_json::from_str(&tool_text(&flat)).unwrap();
    assert!(flat[0].get("children").is_none(), "{flat}");
}

// B:provide_mcp_find_references_tool — verify unit "each reference includes source span"
#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "specforge.find_references returns all reference locations"
)]
fn find_references_returns_source_spans() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // One reference: beta, declared at test.spec lines 10-15.
    assert_eq!(
        parsed["locations"],
        json!([{
            "referencing_entity_id": "beta",
            "source_span": {
                "file": "test.spec",
                "start_line": 10,
                "start_col": 0,
                "end_line": 15,
                "end_col": 0
            }
        }])
    );
}

// B:provide_mcp_outline_tool — verify unit "outline entries sorted by line number"
#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "outline entries sorted by line number"
)]
fn outline_sorted_by_line_extended() {
    let mut server = server_with_unordered_file();
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "order.spec"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let lines: Vec<u64> = parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["range"]["start_line"].as_u64().unwrap())
        .collect();
    assert_eq!(lines, vec![5, 12, 20]);
}
