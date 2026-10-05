use crate::runtime::{WasmCallResult, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use std::collections::HashSet;

/// Auto-promoted MCP tool derived from a CLI command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoPromotedMcpTool {
    pub name: String,
    pub source_command_id: String,
    pub input_schema: serde_json::Value,
}

/// Convert CLI commands into MCP tools by deriving input schemas from args.
/// Returns auto-promoted tools. Explicit MCP tools with same name produce I017 info.
/// Tool names follow the `specforge.{ext_short}.{cmd_id}` convention.
pub fn auto_promote_commands_to_mcp_tools(
    commands: &[(&str, &[(&str, &str)])], // (command_id, [(arg_name, arg_type)])
    explicit_tool_names: &HashSet<String>,
    ext_short: &str,
) -> (Vec<AutoPromotedMcpTool>, Vec<Diagnostic>) {
    let mut tools = Vec::new();
    let mut diagnostics = Vec::new();

    for (id, args) in commands {
        let tool_name = format!("specforge.{}.{}", ext_short, id);

        if explicit_tool_names.contains(&tool_name) {
            diagnostics.push(Diagnostic {
                code: "I017".to_string(),
                severity: Severity::Info,
                message: format!(
                    "command '{}' not auto-promoted: explicit MCP tool '{}' already exists",
                    id, tool_name
                ),
                span: None,
                suggestion: None,
                data: None,
            });
            continue;
        }

        let mut properties = serde_json::Map::new();
        for (arg_name, arg_type) in *args {
            let schema_type = match *arg_type {
                "string" | "path" => "string",
                "bool" => "boolean",
                "integer" => "integer",
                "enum" => "string",
                _ => "string",
            };
            properties.insert(
                arg_name.to_string(),
                serde_json::json!({"type": schema_type}),
            );
        }

        let input_schema = serde_json::json!({
            "type": "object",
            "properties": properties,
        });

        tools.push(AutoPromotedMcpTool {
            name: tool_name,
            source_command_id: id.to_string(),
            input_schema,
        });
    }

    (tools, diagnostics)
}

/// Output of a surface command dispatch.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Dispatch a surface command by calling its Wasm export.
pub fn dispatch_surface_command(
    extension_name: &str,
    export_name: &str,
    args_json: &[u8],
    runtime: &dyn WasmRuntime,
) -> Result<CommandOutput, Diagnostic> {
    match runtime.call_export(extension_name, export_name, args_json) {
        WasmCallResult::Ok(output) => {
            // Parse output as JSON with exit_code, stdout, stderr
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output) {
                Ok(CommandOutput {
                    exit_code: val.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    stdout: val
                        .get("stdout")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .as_bytes()
                        .to_vec(),
                    stderr: val
                        .get("stderr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .as_bytes()
                        .to_vec(),
                })
            } else {
                Ok(CommandOutput {
                    exit_code: 0,
                    stdout: output,
                    stderr: vec![],
                })
            }
        }
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "surface command {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
            data: None,
        }),
    }
}

/// Dispatch an MCP tool by calling its Wasm export.
pub fn dispatch_surface_mcp_tool(
    extension_name: &str,
    export_name: &str,
    input_json: &[u8],
    runtime: &dyn WasmRuntime,
) -> Result<serde_json::Value, Diagnostic> {
    match runtime.call_export(extension_name, export_name, input_json) {
        WasmCallResult::Ok(output) => serde_json::from_slice(&output).map_err(|e| Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!("MCP tool {}() returned invalid JSON: {}", export_name, e),
            span: None,
            suggestion: None,
            data: None,
        }),
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "MCP tool {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
            data: None,
        }),
    }
}

/// Dispatch an MCP resource by calling its Wasm export.
pub fn dispatch_surface_mcp_resource(
    extension_name: &str,
    export_name: &str,
    uri: &str,
    runtime: &dyn WasmRuntime,
) -> Result<(Vec<u8>, String), Diagnostic> {
    let input = serde_json::json!({"uri": uri});
    let input_bytes = serde_json::to_vec(&input).unwrap();

    match runtime.call_export(extension_name, export_name, &input_bytes) {
        WasmCallResult::Ok(output) => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output) {
                let content = val
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .as_bytes()
                    .to_vec();
                let mime_type = val
                    .get("mime_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("application/octet-stream")
                    .to_string();
                Ok((content, mime_type))
            } else {
                Ok((output, "application/octet-stream".to_string()))
            }
        }
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E028".to_string(),
            severity: Severity::Error,
            message: format!(
                "MCP resource {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MockRuntime, WasmTrapInfo};

    // -- auto_promote_commands_to_mcp_tools --

    // B:auto_promote_commands — verify unit "CLI command auto-promoted to MCP tool"
    #[test]
    fn test_auto_promote_cli_command() {
        let args = vec![("path", "path"), ("verbose", "bool")];
        let commands = [("analyze", args.as_slice())];
        let (tools, diags) =
            auto_promote_commands_to_mcp_tools(&commands, &HashSet::new(), "software");
        assert!(diags.is_empty());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "specforge.software.analyze");
        assert_eq!(tools[0].source_command_id, "analyze");

        let props = tools[0].input_schema.get("properties").unwrap();
        assert!(props.get("path").is_some());
        assert!(props.get("verbose").is_some());
    }

    // B:auto_promote_commands — verify unit "derived input_schema computed from command args"
    #[test]
    fn test_auto_promote_derived_schema() {
        let args = vec![("output", "string")];
        let commands = [("report", args.as_slice())];
        let (tools, _) = auto_promote_commands_to_mcp_tools(&commands, &HashSet::new(), "software");
        let props = tools[0].input_schema.get("properties").unwrap();
        let output_type = props
            .get("output")
            .unwrap()
            .get("type")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(output_type, "string");
    }

    // B:auto_promote_commands — verify unit "explicit MCP tool wins over auto-promoted (I017)"
    #[test]
    fn test_auto_promote_explicit_tool_wins() {
        let commands = [("analyze", [].as_slice())];
        let explicit = HashSet::from(["specforge.software.analyze".to_string()]);
        let (tools, diags) = auto_promote_commands_to_mcp_tools(&commands, &explicit, "software");
        assert!(tools.is_empty());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "I017");
    }

    // B:auto_promote_commands — verify unit "command with no args produces empty input_schema"
    #[test]
    fn test_auto_promote_no_args_empty_schema() {
        let commands: &[(&str, &[(&str, &str)])] = &[("check", &[])];
        let (tools, _) = auto_promote_commands_to_mcp_tools(commands, &HashSet::new(), "software");
        let props = tools[0]
            .input_schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(props.is_empty());
    }

    // B:auto_promote_commands — verify contract
    #[test]
    fn test_auto_promote_contract() {
        // ensures: promoted tools follow specforge.{ext_short}.{cmd_id} naming
        let commands: &[(&str, &[(&str, &str)])] = &[("run", &[("target", "string")])];
        let (tools, diags) =
            auto_promote_commands_to_mcp_tools(commands, &HashSet::new(), "coverage");
        assert_eq!(tools.len(), 1);
        assert!(tools[0].name.starts_with("specforge.coverage."));
        assert!(diags.is_empty());

        // ensures: explicit wins with I017
        let explicit = HashSet::from(["specforge.coverage.run".to_string()]);
        let (tools, diags) = auto_promote_commands_to_mcp_tools(commands, &explicit, "coverage");
        assert!(tools.is_empty());
        assert!(diags.iter().any(|d| d.code == "I017"));
    }

    // -- dispatch_surface_command --

    // B:dispatch_surface_command — verify unit "args serialized as JSON to cmd__ export"
    #[test]
    fn test_dispatch_command_args_serialized() {
        let output = serde_json::json!({"exit_code": 0, "stdout": "ok", "stderr": ""});
        let runtime =
            MockRuntime::new().with_call_ok("cmd__analyze", serde_json::to_vec(&output).unwrap());

        let result = dispatch_surface_command("@ext/a", "cmd__analyze", b"{}", &runtime);
        assert!(result.is_ok());
        let cmd_output = result.unwrap();
        assert_eq!(cmd_output.exit_code, 0);
        assert_eq!(cmd_output.stdout, b"ok");
    }

    // B:dispatch_surface_command — verify unit "Wasm trap produces ExtensionError"
    #[test]
    fn test_dispatch_command_trap_produces_error() {
        let runtime = MockRuntime::new().with_call_trap(
            "cmd__analyze",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "panic in command".to_string(),
                export_name: "cmd__analyze".to_string(),
            },
        );

        let result = dispatch_surface_command("@ext/a", "cmd__analyze", b"{}", &runtime);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E028");
    }

    // B:dispatch_surface_command — verify unit "exit_code, stdout, stderr returned"
    #[test]
    fn test_dispatch_command_returns_output_fields() {
        let output = serde_json::json!({"exit_code": 42, "stdout": "output", "stderr": "warn"});
        let runtime =
            MockRuntime::new().with_call_ok("cmd__report", serde_json::to_vec(&output).unwrap());

        let result = dispatch_surface_command("@ext/a", "cmd__report", b"{}", &runtime).unwrap();
        assert_eq!(result.exit_code, 42);
        assert_eq!(result.stdout, b"output");
        assert_eq!(result.stderr, b"warn");
    }

    // -- dispatch_surface_mcp_tool --

    // B:dispatch_surface_mcp_tool — verify unit "JSON passed to mcp__ export"
    #[test]
    fn test_dispatch_mcp_tool_json_passed() {
        let output = serde_json::json!({"result": "found"});
        let runtime =
            MockRuntime::new().with_call_ok("mcp__search", serde_json::to_vec(&output).unwrap());

        let result =
            dispatch_surface_mcp_tool("@ext/a", "mcp__search", b"{\"query\":\"x\"}", &runtime);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["result"], "found");
    }

    // B:dispatch_surface_mcp_tool — verify unit "Wasm trap produces structured MCP error"
    #[test]
    fn test_dispatch_mcp_tool_trap() {
        let runtime = MockRuntime::new().with_call_trap(
            "mcp__search",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "panic".to_string(),
                export_name: "mcp__search".to_string(),
            },
        );

        let result = dispatch_surface_mcp_tool("@ext/a", "mcp__search", b"{}", &runtime);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E028");
    }

    // B:dispatch_surface_mcp_tool — verify unit "output returned as tool result"
    #[test]
    fn test_dispatch_mcp_tool_output_returned() {
        let output = serde_json::json!({"entities": [{"id": "b1"}]});
        let runtime =
            MockRuntime::new().with_call_ok("mcp__graph", serde_json::to_vec(&output).unwrap());

        let val = dispatch_surface_mcp_tool("@ext/a", "mcp__graph", b"{}", &runtime).unwrap();
        assert_eq!(val["entities"][0]["id"], "b1");
    }

    // -- dispatch_surface_mcp_resource --

    // B:dispatch_surface_mcp_resource — verify unit "content + mime_type returned"
    #[test]
    fn test_dispatch_mcp_resource_content_returned() {
        let output = serde_json::json!({"content": "graph data", "mime_type": "application/json"});
        let runtime = MockRuntime::new()
            .with_call_ok("mcp__spec_graph", serde_json::to_vec(&output).unwrap());

        let (content, mime) =
            dispatch_surface_mcp_resource("@ext/a", "mcp__spec_graph", "spec://graph", &runtime)
                .unwrap();
        assert_eq!(content, b"graph data");
        assert_eq!(mime, "application/json");
    }

    // B:dispatch_surface_mcp_resource — verify unit "Wasm trap produces MCP error"
    #[test]
    fn test_dispatch_mcp_resource_trap() {
        let runtime = MockRuntime::new().with_call_trap(
            "mcp__graph",
            WasmTrapInfo {
                kind: "unreachable".to_string(),
                message: "panic".to_string(),
                export_name: "mcp__graph".to_string(),
            },
        );

        let result =
            dispatch_surface_mcp_resource("@ext/a", "mcp__graph", "spec://graph", &runtime);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E028");
    }

    // B:dispatch — verify contract "dispatch contracts"
    #[test]
    fn test_dispatch_contracts() {
        // Command dispatch contract
        let output = serde_json::json!({"exit_code": 0, "stdout": "ok", "stderr": ""});
        let runtime =
            MockRuntime::new().with_call_ok("cmd__test", serde_json::to_vec(&output).unwrap());
        let cmd = dispatch_surface_command("@ext/a", "cmd__test", b"{}", &runtime).unwrap();
        assert_eq!(cmd.exit_code, 0);

        // MCP tool dispatch contract
        let tool_out = serde_json::json!({"ok": true});
        let runtime2 =
            MockRuntime::new().with_call_ok("mcp__tool", serde_json::to_vec(&tool_out).unwrap());
        let tool_val = dispatch_surface_mcp_tool("@ext/a", "mcp__tool", b"{}", &runtime2).unwrap();
        assert_eq!(tool_val["ok"], true);

        // Resource dispatch contract
        let res_out = serde_json::json!({"content": "data", "mime_type": "text/plain"});
        let runtime3 =
            MockRuntime::new().with_call_ok("mcp__res", serde_json::to_vec(&res_out).unwrap());
        let (content, mime) =
            dispatch_surface_mcp_resource("@ext/a", "mcp__res", "spec://res", &runtime3).unwrap();
        assert_eq!(content, b"data");
        assert_eq!(mime, "text/plain");
    }
}
