use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommandArgType {
    #[serde(rename = "string")]
    StringArg,
    #[serde(rename = "path")]
    PathArg,
    #[serde(rename = "bool")]
    BoolArg,
    #[serde(rename = "enum")]
    EnumArg { values: Vec<String> },
    #[serde(rename = "integer")]
    IntegerArg,
}

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
    pub enabled: bool,
}

/// Register surface contributions from manifests.
/// Detects duplicate command IDs and MCP tool names across extensions (E039).
pub fn register_surface_contributions(
    manifests: &[(String, Option<SurfaceContributions>)],
) -> (Vec<SurfaceRegistryEntry>, Vec<Diagnostic>) {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen_commands: HashMap<String, String> = HashMap::new();
    let mut seen_tools: HashMap<String, String> = HashMap::new();
    let mut seen_resources: HashMap<String, String> = HashMap::new();

    for (ext_name, surfaces) in manifests {
        let Some(surfaces) = surfaces else { continue };

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
                });
            } else {
                seen_commands.insert(cmd.id.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::Command,
                    contribution_name: cmd.id.clone(),
                    extension_name: ext_name.clone(),
                    export_name: cmd.export.clone(),
                    enabled: true,
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
                });
            } else {
                seen_tools.insert(tool.name.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::McpTool,
                    contribution_name: tool.name.clone(),
                    extension_name: ext_name.clone(),
                    export_name: tool.export.clone(),
                    enabled: true,
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
                });
            } else {
                seen_resources.insert(resource.name.clone(), ext_name.clone());
                entries.push(SurfaceRegistryEntry {
                    surface_type: SurfaceType::McpResource,
                    contribution_name: resource.name.clone(),
                    extension_name: ext_name.clone(),
                    export_name: resource.export.clone(),
                    enabled: true,
                });
            }
        }
    }

    (entries, diagnostics)
}
