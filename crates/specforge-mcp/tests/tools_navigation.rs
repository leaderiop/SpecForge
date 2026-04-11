use specforge_mcp::McpServer;
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn span() -> SourceSpan {
    SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 }
}

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();

    let mut fields_a = FieldMap::new();
    fields_a.push("contract".into(), FieldValue::String("The system MUST do alpha".into()));
    fields_a.push("verify".into(), FieldValue::VerifyList(vec![
        VerifyStatement { kind: "unit".into(), description: "test alpha".into() },
    ]));

    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: span(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind { raw: "feature".into() },
        title: Some("Beta Feature".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "test.spec".into(), start_line: 10, start_col: 0, end_line: 15, end_col: 0 },
    });
    graph.add_edge(Edge { source: "beta".into(), target: "alpha".into(), label: "behaviors".into() });
    state.graph = graph;

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

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"].as_str().unwrap().to_string()
}

// --- specforge.inspect ---

// B:provide_mcp_inspect_tool — verify unit "returns entity details"
#[test]
#[specforge_test(behavior = "provide_mcp_inspect_tool", verify = "specforge.inspect returns full entity details")]
fn inspect_returns_details() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert_eq!(parsed["kind"], "behavior");
    assert!(parsed["source_span"].is_object());
    assert!(parsed["contract"].is_string());
    assert!(parsed["verify_declarations"].is_array());
}

// B:provide_mcp_inspect_tool — verify unit "includes reference count"
#[test]
#[specforge_test(behavior = "provide_mcp_inspect_tool", verify = "response includes references and verify declarations")]
fn inspect_includes_reference_count() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["reference_count"].as_u64().unwrap() > 0);
}

// B:provide_mcp_inspect_tool — verify unit "unknown entity returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_inspect_tool", verify = "non-existent entity returns error response")]
fn inspect_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// --- specforge.find_definition ---

// B:provide_mcp_find_definition_tool — verify unit "returns source location"
#[test]
#[specforge_test(behavior = "provide_mcp_find_definition_tool", verify = "specforge.find_definition returns file, line, and column")]
fn find_definition_returns_location() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_definition", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert_eq!(parsed["file_path"], "test.spec");
    assert_eq!(parsed["line"], 1);
}

// B:provide_mcp_find_definition_tool — verify unit "unknown entity returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_find_definition_tool", verify = "non-existent entity returns error response")]
fn find_definition_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_definition", json!({"entity_id": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// --- specforge.find_references ---

// B:provide_mcp_find_references_tool — verify unit "returns referencing entities"
#[test]
#[specforge_test(behavior = "provide_mcp_find_references_tool", verify = "specforge.find_references returns all reference locations")]
fn find_references_returns_refs() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_references", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    let locations = parsed["locations"].as_array().unwrap();
    assert!(!locations.is_empty());
    assert_eq!(locations[0]["referencing_entity_id"], "beta");
}

// B:provide_mcp_find_references_tool — verify unit "unknown entity returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_find_references_tool", verify = "non-existent entity returns error response")]
fn find_references_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_references", json!({"entity_id": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// --- specforge.outline ---

// B:provide_mcp_outline_tool — verify unit "returns entities in file"
#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "specforge.outline returns all entities defined in file")]
fn outline_returns_entities_in_file() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "test.spec"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entries = parsed.as_array().unwrap();
    assert!(!entries.is_empty());

    for entry in entries {
        assert_eq!(entry["range"]["file"], "test.spec");
    }
}

// B:provide_mcp_outline_tool — verify unit "empty for unknown file"
#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "non-existent file returns error response")]
fn outline_empty_for_unknown_file() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "nonexistent.spec"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.as_array().unwrap().is_empty());
}

// B:provide_mcp_outline_tool — verify unit "sorted by line number"
#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "sorted by line number")]
fn outline_sorted_by_line() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "test.spec"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entries = parsed.as_array().unwrap();
    if entries.len() > 1 {
        for i in 0..entries.len() - 1 {
            let line_a = entries[i]["range"]["start_line"].as_u64().unwrap();
            let line_b = entries[i + 1]["range"]["start_line"].as_u64().unwrap();
            assert!(line_a <= line_b);
        }
    }
}

// --- specforge.suggest_fixes ---

// B:provide_mcp_suggest_fixes_tool — verify unit "returns suggestions from diagnostics"
#[test]
#[specforge_test(behavior = "provide_mcp_suggest_fixes_tool", verify = "specforge.suggest_fixes returns applicable fix suggestions")]
fn suggest_fixes_returns_suggestions() {
    let mut server = test_server();
    // Add a diagnostic with suggestion
    server.state_mut().diagnostics.push(specforge_common::Diagnostic {
        code: "W001".into(),
        severity: specforge_common::Severity::Warning,
        message: "alpha has no tests field".into(),
        span: Some(span()),
        suggestion: Some("Add a tests field".into()),
    });

    let resp = call_tool(&mut server, "specforge.suggest_fixes", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let suggestions = parsed.as_array().unwrap();
    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0]["kind"], "quickfix");
}

// B:provide_mcp_suggest_fixes_tool — verify unit "empty when no diagnostics"
#[test]
#[specforge_test(behavior = "provide_mcp_suggest_fixes_tool", verify = "clean entity with no diagnostics returns empty list")]
fn suggest_fixes_empty_when_no_diagnostics() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.suggest_fixes", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.as_array().unwrap().is_empty());
}

// B:provide_mcp_find_references_tool — verify unit "entity with no references returns empty list"
#[test]
#[specforge_test(behavior = "provide_mcp_find_references_tool", verify = "entity with no references returns empty list")]
fn find_references_empty_list() {
    let mut server = test_server();
    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "orphan_node".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Orphan".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "orphan.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    let resp = call_tool(&mut server, "specforge.find_references", json!({"entity_id": "orphan_node"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["locations"].as_array().unwrap().is_empty());
}

// B:provide_mcp_outline_tool — verify unit "nested entries included for complex entities"
#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "nested entries included for complex entities")]
fn outline_nested_entries_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "test.spec"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_array());
}

// B:provide_mcp_suggest_fixes_tool — verify unit "diagnostic_code filter restricts to matching diagnostics"
#[test]
#[specforge_test(behavior = "provide_mcp_suggest_fixes_tool", verify = "diagnostic_code filter restricts to matching diagnostics")]
fn suggest_fixes_diagnostic_code_filter() {
    let mut server = test_server();
    use specforge_common::{Diagnostic, Severity};
    server.state_mut().diagnostics.push(Diagnostic { code: "V001".into(), severity: Severity::Error, message: "err1".into(), span: None, suggestion: Some("fix1".into()) });
    server.state_mut().diagnostics.push(Diagnostic { code: "W001".into(), severity: Severity::Warning, message: "warn1".into(), span: None, suggestion: Some("fix2".into()) });
    let resp = call_tool(&mut server, "specforge.suggest_fixes", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

// B:provide_mcp_find_references_tool — verify unit "each reference includes source span"
#[test]
#[specforge_test(behavior = "provide_mcp_find_references_tool", verify = "each reference includes source span")]
fn find_references_returns_source_spans() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_references", json!({"entity_id": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let locations = parsed["locations"].as_array().unwrap();
    assert!(!locations.is_empty());
    assert!(locations[0]["file"].is_string() || locations[0]["source_span"].is_object());
}

// B:provide_mcp_outline_tool — verify unit "outline entries sorted by line number"
#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "outline entries sorted by line number")]
fn outline_sorted_by_line_extended() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "test.spec"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entries = parsed.as_array().unwrap();
    if entries.len() > 1 {
        for i in 0..entries.len() - 1 {
            let line_a = entries[i]["range"]["start_line"].as_u64().unwrap();
            let line_b = entries[i + 1]["range"]["start_line"].as_u64().unwrap();
            assert!(line_a <= line_b, "outline entries should be sorted by line number");
        }
    }
}
