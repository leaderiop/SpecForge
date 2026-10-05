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

#[cfg(test)]
mod tests {
    use super::*;

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
}
