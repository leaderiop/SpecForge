use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
use specforge_protocol_types::SurfaceDescriptor;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceContributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub mcp_tools: Vec<McpToolContribution>,
    #[serde(default)]
    pub mcp_resources: Vec<McpResourceContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    pub export: String,
    #[serde(default)]
    pub args: Vec<CommandArg>,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandArg {
    pub name: String,
    pub arg_type: CommandArgType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// The protocol's own type: one wire shape (`"string"`, `{"enum":
/// {"values": [..]}}`, ...) for manifests and describe payloads alike.
pub use specforge_protocol_types::CommandArgType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolContribution {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    pub export: String,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceContribution {
    pub uri_template: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub export: String,
    pub mime_type: String,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceSandboxOverride {
    #[serde(default)]
    pub fs_read: Option<bool>,
    #[serde(default)]
    pub fs_write: Option<bool>,
    #[serde(default)]
    pub network: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceType {
    Command,
    McpTool,
    McpResource,
    AutoPromotedTool,
}

#[derive(Debug, Clone)]
pub struct SurfaceRegistryEntry {
    pub surface_type: SurfaceType,
    pub contribution_name: String,
    pub extension_name: String,
    pub export_name: String,
}

/// Refuse each explicit MCP tool of `ext_name` whose `input_schema` or
/// `output_schema` is not a JSON object (E055): it is removed from
/// `surfaces`, so it is neither registered nor listed.
pub fn refuse_malformed_tool_schemas(
    ext_name: &str,
    surfaces: &mut SurfaceDescriptor,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    surfaces.mcp_tools.retain(|tool| {
        let malformed: Vec<&str> = [
            ("input_schema", Some(&tool.input_schema)),
            ("output_schema", tool.output_schema.as_ref()),
        ]
        .into_iter()
        .filter(|(_, schema)| schema.is_some_and(|s| !s.is_object()))
        .map(|(name, _)| name)
        .collect();
        for name in &malformed {
            diagnostics.push(Diagnostic {
                code: "E055".to_string(),
                severity: Severity::Error,
                message: format!(
                    "MCP tool '{}' of extension '{}': {} must be a JSON object; the tool is not registered",
                    tool.name, ext_name, name
                ),
                span: None,
                suggestion: Some("declare the schema as a JSON Schema object".to_string()),
                data: None,
            });
        }
        malformed.is_empty()
    });
    diagnostics
}

/// Register the declared surfaces, extension by extension in load order
/// (first registration wins). Detects duplicate command IDs, MCP tool
/// names and MCP resource names across extensions (E039).
pub fn register_surface_contributions(
    declared: &[(String, SurfaceDescriptor)],
) -> (Vec<SurfaceRegistryEntry>, Vec<Diagnostic>) {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen_commands: HashMap<String, String> = HashMap::new();
    let mut seen_tools: HashMap<String, String> = HashMap::new();
    let mut seen_resources: HashMap<String, String> = HashMap::new();

    for (ext_name, surfaces) in declared {
        for cmd in &surfaces.commands {
            if let Some(first_ext) = seen_commands.get(&cmd.id) {
                diagnostics.push(Diagnostic {
                    code: "E039".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "duplicate surface command ID '{}': extension '{}' conflicts with '{}'",
                        cmd.id, ext_name, first_ext
                    ),
                    span: None,
                    suggestion: None,
                    data: None,
                });
            } else {
                seen_commands.insert(cmd.id.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::Command,
                    contribution_name: cmd.id.clone(),
                    extension_name: ext_name.clone(),
                    export_name: cmd.export.clone(),
                });
            }
        }

        for tool in &surfaces.mcp_tools {
            if let Some(first_ext) = seen_tools.get(&tool.name) {
                diagnostics.push(Diagnostic {
                    code: "E039".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "duplicate MCP tool name '{}': extension '{}' conflicts with '{}'",
                        tool.name, ext_name, first_ext
                    ),
                    span: None,
                    suggestion: None,
                    data: None,
                });
            } else {
                seen_tools.insert(tool.name.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::McpTool,
                    contribution_name: tool.name.clone(),
                    extension_name: ext_name.clone(),
                    export_name: tool.export.clone(),
                });
            }
        }

        for resource in &surfaces.mcp_resources {
            if let Some(first_ext) = seen_resources.get(&resource.name) {
                diagnostics.push(Diagnostic {
                    code: "E039".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "duplicate MCP resource name '{}': extension '{}' conflicts with '{}'",
                        resource.name, ext_name, first_ext
                    ),
                    span: None,
                    suggestion: None,
                    data: None,
                });
            } else {
                seen_resources.insert(resource.name.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::McpResource,
                    contribution_name: resource.name.clone(),
                    extension_name: ext_name.clone(),
                    export_name: resource.export.clone(),
                });
            }
        }
    }

    (entries, diagnostics)
}
