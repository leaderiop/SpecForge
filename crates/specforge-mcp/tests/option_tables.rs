//! Each enumerated argument a core tool takes is advertised from its option
//! table (ADR 0027): the input schema's `enum` is every name the table
//! accepts (listed names, then aliases), its `default` the table's, and the
//! description names each choice. The tools that parse a format with a
//! table accept its names and aliases and refuse any other name.

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_ops::export::{AGENT_FORMAT, FORMAT};
use specforge_ops::model::{
    DEPS, GROUP_BY, MODEL_FIELDS, MODEL_FORMAT, OUTLINE_FIELDS, OUTLINE_FORMAT,
};
use specforge_ops::options::OptionTable;

/// What a table says an input-schema property must hold.
struct Advertised {
    tool: &'static str,
    argument: &'static str,
    accepted: Vec<&'static str>,
    names: Vec<&'static str>,
    default: Option<&'static str>,
}

fn advertised<T: Copy + PartialEq>(
    tool: &'static str,
    argument: &'static str,
    table: &OptionTable<T>,
) -> Advertised {
    Advertised {
        tool,
        argument,
        accepted: table.accepted().collect(),
        names: table.names().collect(),
        default: table.default_name(),
    }
}

/// [`advertised`] of a required argument: no default.
fn required<T: Copy + PartialEq>(
    tool: &'static str,
    argument: &'static str,
    table: &OptionTable<T>,
) -> Advertised {
    Advertised {
        default: None,
        ..advertised(tool, argument, table)
    }
}

/// Every enumerated argument of a core tool, with the table it reads.
fn enumerated() -> Vec<Advertised> {
    vec![
        advertised("specforge.query", "format", &AGENT_FORMAT),
        advertised("specforge.export", "format", &AGENT_FORMAT),
        required("specforge.render", "format", &FORMAT),
        advertised("specforge.model", "format", &MODEL_FORMAT),
        advertised("specforge.model", "group_by", &GROUP_BY),
        advertised("specforge.model", "fields", &MODEL_FIELDS),
        advertised("specforge.outline_extensions", "format", &OUTLINE_FORMAT),
        advertised("specforge.outline_extensions", "fields", &OUTLINE_FIELDS),
        advertised("specforge.outline_extensions", "deps", &DEPS),
    ]
}

/// The input schema property `argument` of core tool `tool`.
fn property(tool: &str, argument: &str) -> Value {
    let spec = specforge_mcp::tools::CORE_TOOLS
        .iter()
        .find(|spec| spec.name == tool)
        .unwrap_or_else(|| panic!("no core tool {tool}"));
    (spec.schema)()["properties"][argument].clone()
}

#[specforge_test_macros::test(
    behavior = "name_enumerated_options_once",
    verify = "each enumerated MCP argument advertises the table's names and default"
)]
fn each_enumerated_argument_advertises_its_table() {
    for expected in enumerated() {
        let at = format!("{}.{}", expected.tool, expected.argument);
        let property = property(expected.tool, expected.argument);
        assert_eq!(property["type"], "string", "{at}");
        let listed: Vec<&str> = property["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("{at} lists no enum: {property}"))
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect();
        assert_eq!(listed, expected.accepted, "{at}");
        assert_eq!(
            property.get("default").and_then(Value::as_str),
            expected.default,
            "{at}: one default, the table's, on every surface"
        );
        let description = property["description"].as_str().unwrap_or_default();
        for name in &expected.names {
            assert!(description.contains(name), "{at}: {description}");
        }
    }
}

/// A server serving a project on disk: one behavior `alpha`, the software
/// extension. The directory outlives the server (the test process exits).
fn served() -> McpServer {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("main.spec"),
        "behavior alpha \"Alpha\" {\n  contract \"The system MUST work\"\n}\n",
    )
    .unwrap();
    let root = dir.keep();
    let mut server = McpServer::with_project_root(root);
    let init = json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
        "protocolVersion": "2025-03-26", "capabilities": {},
        "clientInfo": {"name": "option_tables", "version": "0"}}});
    server.handle_message(&init.to_string());
    server
}

/// The `tools/call` response for `name` with `arguments`.
fn call(server: &mut McpServer, name: &str, arguments: Value) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}});
    let response = server.handle_message(&request.to_string()).unwrap();
    serde_json::from_str(&response).unwrap()
}

/// A successful call's result, its text read as JSON.
fn content(response: &Value) -> Value {
    assert_ne!(response["result"]["isError"], true, "{response}");
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_render_tool",
    verify = "graph and its alias json select the full graph renderer"
)]
fn render_accepts_graph_and_its_json_alias() {
    let mut server = served();
    let mut written = Vec::new();
    for name in ["graph", "json"] {
        let out = tempfile::TempDir::new().unwrap();
        let result = content(&call(
            &mut server,
            "specforge.render",
            json!({"format": name, "out_dir": out.path().to_str().unwrap()}),
        ));
        assert_eq!(result["format"], "graph", "{name}: {result}");
        let file = out.path().join("graph.json");
        assert_eq!(
            result["output_files"],
            json!([file.display().to_string()]),
            "{name}"
        );
        let graph: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            graph["format_version"], "2.0",
            "the full graph export: {graph}"
        );
        written.push(graph);
    }
    assert_eq!(written[0], written[1], "json is graph");
}

#[specforge_test_macros::test(
    behavior = "provide_mcp_query_tool",
    verify = "an unknown format is an invalid-input error naming the expected formats"
)]
fn query_refuses_an_unknown_format() {
    let mut server = served();
    for (name, message) in [
        (
            "yaml",
            "Unknown format: yaml. Expected: graph, context, brief",
        ),
        // dot is the render tool's.
        (
            "dot",
            "Unknown format: dot. Expected: graph, context, brief",
        ),
    ] {
        let response = call(
            &mut server,
            "specforge.query",
            json!({"entity_id": "alpha", "format": name}),
        );
        let error = crate::tool_errors::mcp_error(&response);
        assert_eq!(error["code"], "invalid_input", "{error}");
        assert_eq!(error["argument"], "format", "{error}");
        assert_eq!(error["message"], message);
    }
    // The json alias reads as graph.
    let graph = content(&call(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "graph"}),
    ));
    let json = content(&call(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "json"}),
    ));
    assert_eq!(graph, json);
    assert_eq!(graph["nodes"][0]["id"], "alpha", "{graph}");
}
