use crate::runtime::{WasmCallResult, WasmRuntime};
use specforge_common::{Diagnostic, Severity};
use specforge_registry::SandboxPolicy;
use std::collections::HashSet;

/// Validate that all declared surface exports are present in the Wasm module.
pub fn validate_surface_exports(
    commands: &[(&str, &str)],       // (id, export_name)
    mcp_tools: &[(&str, &str)],      // (name, export_name)
    mcp_resources: &[(&str, &str)],  // (name, export_name)
    available_exports: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (id, export) in commands {
        let expected = format!("cmd__{}", id);
        if !available_exports.contains(&expected) && !available_exports.contains(*export) {
            diagnostics.push(Diagnostic {
                code: "E036".to_string(),
                severity: Severity::Error,
                message: format!(
                    "surface command '{}': export '{}' not found in Wasm module",
                    id, expected
                ),
                span: None,
                suggestion: Some(format!("add #[export_name = \"{}\"] to the Wasm module", expected)),
            });
        }
    }

    for (name, export) in mcp_tools {
        let expected = format!("mcp__{}", name);
        if !available_exports.contains(&expected) && !available_exports.contains(*export) {
            diagnostics.push(Diagnostic {
                code: "E036".to_string(),
                severity: Severity::Error,
                message: format!(
                    "MCP tool '{}': export '{}' not found in Wasm module",
                    name, expected
                ),
                span: None,
                suggestion: Some(format!("add #[export_name = \"{}\"] to the Wasm module", expected)),
            });
        }
    }

    for (name, export) in mcp_resources {
        let expected = format!("mcp__{}", name);
        if !available_exports.contains(&expected) && !available_exports.contains(*export) {
            diagnostics.push(Diagnostic {
                code: "E036".to_string(),
                severity: Severity::Error,
                message: format!(
                    "MCP resource '{}': export '{}' not found in Wasm module",
                    name, expected
                ),
                span: None,
                suggestion: Some(format!("add #[export_name = \"{}\"] to the Wasm module", expected)),
            });
        }
    }

    diagnostics
}

/// Validate MCP tool input/output schemas are valid JSON Schema objects.
pub fn validate_mcp_tool_schemas(
    tools: &[(&str, &serde_json::Value, Option<&serde_json::Value>)], // (tool_name, input_schema, output_schema)
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (name, input_schema, output_schema) in tools {
        if !input_schema.is_object() {
            diagnostics.push(Diagnostic {
                code: "E037".to_string(),
                severity: Severity::Error,
                message: format!(
                    "MCP tool '{}': input_schema must be a JSON object",
                    name
                ),
                span: None,
                suggestion: Some("provide a valid JSON Schema object".to_string()),
            });
        }
        if let Some(out) = output_schema
            && !out.is_object() {
                diagnostics.push(Diagnostic {
                    code: "E037".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "MCP tool '{}': output_schema must be a JSON object",
                        name
                    ),
                    span: None,
                    suggestion: Some("provide a valid JSON Schema object".to_string()),
                });
            }
    }

    diagnostics
}

/// Validate command argument types are known.
pub fn validate_command_arg_types(
    commands: &[(&str, &[&str])], // (command_id, arg_types)
) -> Vec<Diagnostic> {
    let known_types = ["string", "path", "bool", "enum", "integer"];
    let mut diagnostics = Vec::new();

    for (id, arg_types) in commands {
        for arg_type in *arg_types {
            if !known_types.contains(arg_type) {
                diagnostics.push(Diagnostic {
                    code: "E038".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "command '{}': unknown argument type '{}'",
                        id, arg_type
                    ),
                    span: None,
                    suggestion: Some(format!("known types: {}", known_types.join(", "))),
                });
            }
        }
    }

    diagnostics
}

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
            code: "E041".to_string(),
            severity: Severity::Error,
            message: format!(
                "surface command {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
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
        WasmCallResult::Ok(output) => {
            serde_json::from_slice(&output).map_err(|e| Diagnostic {
                code: "E041".to_string(),
                severity: Severity::Error,
                message: format!(
                    "MCP tool {}() returned invalid JSON: {}",
                    export_name, e
                ),
                span: None,
                suggestion: None,
            })
        }
        WasmCallResult::Trap(trap) => Err(Diagnostic {
            code: "E041".to_string(),
            severity: Severity::Error,
            message: format!(
                "MCP tool {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
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
            code: "E041".to_string(),
            severity: Severity::Error,
            message: format!(
                "MCP resource {}() trapped: {} — {}",
                export_name, trap.kind, trap.message
            ),
            span: None,
            suggestion: None,
        }),
    }
}

/// Enforce surface sandbox: intersect override with extension ceiling.
/// Override cannot expand beyond extension policy.
pub fn enforce_surface_sandbox(
    override_: &SurfaceSandboxOverrideValues,
    extension_policy: &SandboxPolicy,
) -> EffectiveSandbox {
    let fs_read = match (override_.fs_read, extension_policy.file_system_access) {
        (Some(true), Some(false)) => false, // Cannot expand beyond ceiling
        (Some(v), _) => v,
        (None, Some(v)) => v,
        (None, None) => true,
    };

    let fs_write = match (override_.fs_write, extension_policy.file_system_access) {
        (Some(true), Some(false)) => false,
        (Some(v), _) => v,
        (None, Some(v)) => v,
        (None, None) => false,
    };

    let network = match (override_.network, extension_policy.network_access) {
        (Some(true), Some(false)) => false,
        (Some(v), _) => v,
        (None, Some(v)) => v,
        (None, None) => false,
    };

    EffectiveSandbox {
        fs_read,
        fs_write,
        network,
    }
}

/// Surface sandbox override values (simplified for enforcement).
#[derive(Debug, Clone, Default)]
pub struct SurfaceSandboxOverrideValues {
    pub fs_read: Option<bool>,
    pub fs_write: Option<bool>,
    pub network: Option<bool>,
}

/// Effective sandbox after enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSandbox {
    pub fs_read: bool,
    pub fs_write: bool,
    pub network: bool,
}

/// Enforce that MCP resources never get fs_write.
pub fn enforce_resource_sandbox(
    override_: &SurfaceSandboxOverrideValues,
    extension_policy: &SandboxPolicy,
) -> EffectiveSandbox {
    let mut sandbox = enforce_surface_sandbox(override_, extension_policy);
    sandbox.fs_write = false; // Resources are always read-only
    sandbox
}

/// Toggle a surface contribution on/off. Returns true if found.
pub fn toggle_surface_contribution(
    entries: &mut [SurfaceEntry],
    name: &str,
    enabled: bool,
) -> bool {
    let mut found = false;
    for entry in entries.iter_mut() {
        if entry.name == name {
            entry.enabled = enabled;
            found = true;
        }
    }
    found
}

/// Minimal surface entry for toggle tracking.
#[derive(Debug, Clone)]
pub struct SurfaceEntry {
    pub name: String,
    pub surface_type: SurfaceEntryType,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceEntryType {
    Command,
    McpTool,
    McpResource,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{MockRuntime, WasmTrapInfo};

    // -- validate_surface_exports --

    // B:validate_surface_exports — verify unit "all declared cmd exports present passes"
    #[test]
    fn test_validate_all_cmd_exports_present_passes() {
        let exports: HashSet<String> = ["cmd__analyze", "cmd__report"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let commands = [("analyze", "cmd__analyze"), ("report", "cmd__report")];
        let diags = validate_surface_exports(&commands, &[], &[], &exports);
        assert!(diags.is_empty());
    }

    // B:validate_surface_exports — verify unit "missing cmd export produces E036"
    #[test]
    fn test_validate_missing_cmd_export_e036() {
        let exports: HashSet<String> = HashSet::new();
        let commands = [("analyze", "cmd__analyze")];
        let diags = validate_surface_exports(&commands, &[], &[], &exports);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E036");
        assert!(diags[0].message.contains("analyze"));
    }

    // B:validate_surface_exports — verify unit "all declared mcp exports present passes"
    #[test]
    fn test_validate_all_mcp_exports_present_passes() {
        let exports: HashSet<String> = ["mcp__search", "mcp__spec_graph"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let tools = [("search", "mcp__search")];
        let resources = [("spec_graph", "mcp__spec_graph")];
        let diags = validate_surface_exports(&[], &tools, &resources, &exports);
        assert!(diags.is_empty());
    }

    // B:validate_surface_exports — verify unit "missing mcp export produces E036"
    #[test]
    fn test_validate_missing_mcp_export_e036() {
        let exports: HashSet<String> = HashSet::new();
        let tools = [("search", "mcp__search")];
        let diags = validate_surface_exports(&[], &tools, &[], &exports);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E036");
        assert!(diags[0].message.contains("search"));
    }

    // -- validate_mcp_tool_schemas --

    // B:validate_mcp_tool_schemas — verify unit "valid JSON Schema passes"
    #[test]
    fn test_validate_valid_json_schema_passes() {
        let schema = serde_json::json!({"type": "object", "properties": {}});
        let tools = [("search", &schema, None)];
        let diags = validate_mcp_tool_schemas(&tools);
        assert!(diags.is_empty());
    }

    // B:validate_mcp_tool_schemas — verify unit "invalid input_schema produces diagnostic"
    #[test]
    fn test_validate_invalid_schema_produces_diagnostic() {
        let schema = serde_json::json!("not an object");
        let tools = [("search", &schema, None)];
        let diags = validate_mcp_tool_schemas(&tools);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E037");
    }

    // B:validate_mcp_tool_schemas — verify unit "valid output_schema passes"
    #[test]
    fn test_validate_valid_output_schema_passes() {
        let input = serde_json::json!({"type": "object"});
        let output = serde_json::json!({"type": "object", "properties": {"result": {"type": "string"}}});
        let tools = [("search", &input, Some(&output))];
        let diags = validate_mcp_tool_schemas(&tools);
        assert!(diags.is_empty());
    }

    // B:validate_mcp_tool_schemas — verify unit "invalid output_schema produces diagnostic"
    #[test]
    fn test_validate_invalid_output_schema_produces_diagnostic() {
        let input = serde_json::json!({"type": "object"});
        let output = serde_json::json!("not an object");
        let tools = [("search", &input, Some(&output))];
        let diags = validate_mcp_tool_schemas(&tools);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E037");
        assert!(diags[0].message.contains("output_schema"));
    }

    // -- validate_command_arg_types --

    // B:validate_command_arg_types — verify unit "known arg types pass"
    #[test]
    fn test_validate_known_arg_types_pass() {
        let types = vec!["string", "path", "bool", "enum", "integer"];
        let commands = [("analyze", types.as_slice())];
        let diags = validate_command_arg_types(&commands);
        assert!(diags.is_empty());
    }

    // B:validate_surface_exports — verify contract "validation contracts"
    #[test]
    fn test_validate_surface_exports_contract() {
        // ensures: all present → no diagnostics
        let exports: HashSet<String> = ["cmd__a", "mcp__b", "mcp__c"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let diags = validate_surface_exports(
            &[("a", "cmd__a")],
            &[("b", "mcp__b")],
            &[("c", "mcp__c")],
            &exports,
        );
        assert!(diags.is_empty());

        // ensures: missing → E036 with export name
        let diags = validate_surface_exports(
            &[("x", "cmd__x")],
            &[],
            &[],
            &HashSet::new(),
        );
        assert!(diags.iter().all(|d| d.code == "E036"));
    }

    // -- auto_promote_commands_to_mcp_tools --

    // B:auto_promote_commands — verify unit "CLI command auto-promoted to MCP tool"
    #[test]
    fn test_auto_promote_cli_command() {
        let args = vec![("path", "path"), ("verbose", "bool")];
        let commands = [("analyze", args.as_slice())];
        let (tools, diags) = auto_promote_commands_to_mcp_tools(&commands, &HashSet::new(), "software");
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
        let output_type = props.get("output").unwrap().get("type").unwrap().as_str().unwrap();
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
        let props = tools[0].input_schema.get("properties").unwrap().as_object().unwrap();
        assert!(props.is_empty());
    }

    // B:auto_promote_commands — verify contract
    #[test]
    fn test_auto_promote_contract() {
        // ensures: promoted tools follow specforge.{ext_short}.{cmd_id} naming
        let commands: &[(&str, &[(&str, &str)])] = &[("run", &[("target", "string")])];
        let (tools, diags) = auto_promote_commands_to_mcp_tools(commands, &HashSet::new(), "coverage");
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
        let runtime = MockRuntime::new()
            .with_call_ok("cmd__analyze", serde_json::to_vec(&output).unwrap());

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
        assert_eq!(result.unwrap_err().code, "E041");
    }

    // B:dispatch_surface_command — verify unit "exit_code, stdout, stderr returned"
    #[test]
    fn test_dispatch_command_returns_output_fields() {
        let output = serde_json::json!({"exit_code": 42, "stdout": "output", "stderr": "warn"});
        let runtime = MockRuntime::new()
            .with_call_ok("cmd__report", serde_json::to_vec(&output).unwrap());

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
        let runtime = MockRuntime::new()
            .with_call_ok("mcp__search", serde_json::to_vec(&output).unwrap());

        let result = dispatch_surface_mcp_tool("@ext/a", "mcp__search", b"{\"query\":\"x\"}", &runtime);
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
        assert_eq!(result.unwrap_err().code, "E041");
    }

    // B:dispatch_surface_mcp_tool — verify unit "output returned as tool result"
    #[test]
    fn test_dispatch_mcp_tool_output_returned() {
        let output = serde_json::json!({"entities": [{"id": "b1"}]});
        let runtime = MockRuntime::new()
            .with_call_ok("mcp__graph", serde_json::to_vec(&output).unwrap());

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

        let (content, mime) = dispatch_surface_mcp_resource("@ext/a", "mcp__spec_graph", "spec://graph", &runtime).unwrap();
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

        let result = dispatch_surface_mcp_resource("@ext/a", "mcp__graph", "spec://graph", &runtime);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "E041");
    }

    // B:dispatch — verify contract "dispatch contracts"
    #[test]
    fn test_dispatch_contracts() {
        // Command dispatch contract
        let output = serde_json::json!({"exit_code": 0, "stdout": "ok", "stderr": ""});
        let runtime = MockRuntime::new()
            .with_call_ok("cmd__test", serde_json::to_vec(&output).unwrap());
        let cmd = dispatch_surface_command("@ext/a", "cmd__test", b"{}", &runtime).unwrap();
        assert_eq!(cmd.exit_code, 0);

        // MCP tool dispatch contract
        let tool_out = serde_json::json!({"ok": true});
        let runtime2 = MockRuntime::new()
            .with_call_ok("mcp__tool", serde_json::to_vec(&tool_out).unwrap());
        let tool_val = dispatch_surface_mcp_tool("@ext/a", "mcp__tool", b"{}", &runtime2).unwrap();
        assert_eq!(tool_val["ok"], true);

        // Resource dispatch contract
        let res_out = serde_json::json!({"content": "data", "mime_type": "text/plain"});
        let runtime3 = MockRuntime::new()
            .with_call_ok("mcp__res", serde_json::to_vec(&res_out).unwrap());
        let (content, mime) = dispatch_surface_mcp_resource("@ext/a", "mcp__res", "spec://res", &runtime3).unwrap();
        assert_eq!(content, b"data");
        assert_eq!(mime, "text/plain");
    }

    // -- enforce_surface_sandbox --

    // B:enforce_surface_sandbox — verify unit "effective sandbox = intersection of override and ceiling"
    #[test]
    fn test_enforce_sandbox_intersection() {
        let override_ = SurfaceSandboxOverrideValues {
            fs_read: Some(true),
            fs_write: Some(true),
            network: Some(true),
        };
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            network_access: Some(false),
            ..Default::default()
        };
        let effective = enforce_surface_sandbox(&override_, &policy);
        assert!(effective.fs_read);
        assert!(effective.fs_write);
        assert!(!effective.network); // Ceiling denies network
    }

    // B:enforce_surface_sandbox — verify unit "override cannot expand beyond extension policy"
    #[test]
    fn test_enforce_sandbox_cannot_expand() {
        let override_ = SurfaceSandboxOverrideValues {
            fs_read: Some(true),
            fs_write: Some(true),
            network: Some(true),
        };
        let policy = SandboxPolicy {
            file_system_access: Some(false),
            network_access: Some(false),
            ..Default::default()
        };
        let effective = enforce_surface_sandbox(&override_, &policy);
        assert!(!effective.fs_read);
        assert!(!effective.fs_write);
        assert!(!effective.network);
    }

    // B:enforce_surface_sandbox — verify unit "MCP resource fs_write denied regardless"
    #[test]
    fn test_enforce_resource_sandbox_fs_write_denied() {
        let override_ = SurfaceSandboxOverrideValues {
            fs_read: Some(true),
            fs_write: Some(true), // Should still be denied for resources
            network: None,
        };
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            network_access: Some(true),
            ..Default::default()
        };
        let effective = enforce_resource_sandbox(&override_, &policy);
        assert!(effective.fs_read);
        assert!(!effective.fs_write); // Always denied for resources
    }

    // -- toggle_surface_contribution --

    // B:toggle_surface_contribution — verify unit "disabled command excluded"
    #[test]
    fn test_toggle_disabled_command_excluded() {
        let mut entries = vec![
            SurfaceEntry { name: "analyze".to_string(), surface_type: SurfaceEntryType::Command, enabled: true },
            SurfaceEntry { name: "report".to_string(), surface_type: SurfaceEntryType::Command, enabled: true },
        ];
        assert!(toggle_surface_contribution(&mut entries, "analyze", false));
        assert!(!entries[0].enabled);
        assert!(entries[1].enabled);
    }

    // B:toggle_surface_contribution — verify unit "disabled MCP tool excluded"
    #[test]
    fn test_toggle_disabled_mcp_tool_excluded() {
        let mut entries = vec![
            SurfaceEntry { name: "search".to_string(), surface_type: SurfaceEntryType::McpTool, enabled: true },
        ];
        assert!(toggle_surface_contribution(&mut entries, "search", false));
        assert!(!entries[0].enabled);
    }

    // B:toggle_surface_contribution — verify unit "disabled MCP resource excluded"
    #[test]
    fn test_toggle_disabled_mcp_resource_excluded() {
        let mut entries = vec![
            SurfaceEntry { name: "graph".to_string(), surface_type: SurfaceEntryType::McpResource, enabled: true },
        ];
        assert!(toggle_surface_contribution(&mut entries, "graph", false));
        assert!(!entries[0].enabled);
    }

    // B:toggle_surface_contribution — verify unit "re-enabled contribution restored"
    #[test]
    fn test_toggle_reenabled_contribution_restored() {
        let mut entries = vec![
            SurfaceEntry { name: "analyze".to_string(), surface_type: SurfaceEntryType::Command, enabled: false },
        ];
        assert!(toggle_surface_contribution(&mut entries, "analyze", true));
        assert!(entries[0].enabled);
    }

    // B:toggle_surface_contribution + sandbox — verify contract
    #[test]
    fn test_toggle_and_sandbox_contract() {
        // Toggle contract: found returns true, not found returns false
        let mut entries = vec![
            SurfaceEntry { name: "x".to_string(), surface_type: SurfaceEntryType::Command, enabled: true },
        ];
        assert!(toggle_surface_contribution(&mut entries, "x", false));
        assert!(!toggle_surface_contribution(&mut entries, "nonexistent", false));

        // Sandbox contract: intersection semantics
        let override_ = SurfaceSandboxOverrideValues { fs_read: Some(true), fs_write: None, network: Some(false) };
        let policy = SandboxPolicy { file_system_access: Some(true), network_access: Some(true), ..Default::default() };
        let eff = enforce_surface_sandbox(&override_, &policy);
        assert!(eff.fs_read);
        assert!(!eff.network); // Override restricts

        // Resource sandbox: fs_write always denied
        let eff2 = enforce_resource_sandbox(&SurfaceSandboxOverrideValues::default(), &SandboxPolicy::default());
        assert!(!eff2.fs_write);
    }
}
