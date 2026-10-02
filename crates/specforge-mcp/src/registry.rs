use serde_json::{Value, json};
use specforge_registry::{
    CommandArg, CommandArgType, SurfaceContributions, SurfaceRegistryEntry, SurfaceType,
};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use crate::tool::Category;
use crate::types::{McpPromptDescriptor, McpResourceDescriptor, McpToolDescriptor};

pub fn register_defaults(state: &mut McpState) {
    state.resource_registry = default_resources();
    state.tool_registry = default_tools();
    state.prompt_registry = default_prompts();
}

/// Convert manifest surface contributions into MCP tool and resource descriptors,
/// appending them to the existing registries, then auto-promote every CLI
/// command to an MCP tool (see [`auto_promote_commands`]).
pub fn register_extension_surfaces(
    state: &mut McpState,
    manifest_surfaces: &[(String, SurfaceContributions)],
) {
    for (ext_name, surfaces) in manifest_surfaces {
        for tool in &surfaces.mcp_tools {
            state.tool_registry.push(McpToolDescriptor {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                // The manifest's output_schema is the tool's outputSchema.
                output_schema: tool.output_schema.clone(),
                category: Some(extension_category(tool.category.as_deref()).into()),
                source: Some(ext_name.clone()),
                annotations: None,
            });
        }

        for resource in &surfaces.mcp_resources {
            state.resource_registry.push(McpResourceDescriptor {
                uri: resource.uri_template.clone(),
                name: resource.name.clone(),
                description: resource.description.clone(),
                mime_type: Some(resource.mime_type.clone()),
            });
        }
    }
    auto_promote_commands(state, manifest_surfaces);
}

/// Every extension CLI command becomes the MCP tool
/// `specforge.{ext_short}.{cmd_id}`, its input schema derived from the
/// command's args, dispatched to the command's export. A tool already
/// registered under that name (core or explicitly contributed) wins, and
/// the command is reported with I017. Emits `commands_auto_promoted` when
/// any extension contributes commands.
fn auto_promote_commands(
    state: &mut McpState,
    manifest_surfaces: &[(String, SurfaceContributions)],
) {
    let mut promoted_count = 0;
    let mut conflict_count = 0;
    let mut any_commands = false;
    for (ext_name, surfaces) in manifest_surfaces {
        if surfaces.commands.is_empty() {
            continue;
        }
        any_commands = true;
        let explicit: std::collections::HashSet<String> =
            state.tool_registry.iter().map(|t| t.name.clone()).collect();
        let args: Vec<Vec<(&str, &str)>> = surfaces
            .commands
            .iter()
            .map(|cmd| {
                cmd.args
                    .iter()
                    .map(|arg| (arg.name.as_str(), arg_type_name(&arg.arg_type)))
                    .collect()
            })
            .collect();
        let commands: Vec<(&str, &[(&str, &str)])> = surfaces
            .commands
            .iter()
            .zip(&args)
            .map(|(cmd, args)| (cmd.id.as_str(), args.as_slice()))
            .collect();
        let short = ext_short(state, ext_name);
        let (tools, diagnostics) =
            specforge_wasm::auto_promote_commands_to_mcp_tools(&commands, &explicit, &short);
        conflict_count += diagnostics.len();
        state.diagnostics.extend(diagnostics);

        for tool in tools {
            let Some(cmd) = surfaces
                .commands
                .iter()
                .find(|c| c.id == tool.source_command_id)
            else {
                continue;
            };
            // The promoted tool follows its command's enabled state.
            let enabled = state
                .surface_entries
                .iter()
                .find(|e| {
                    e.surface_type == SurfaceType::Command
                        && e.contribution_name == cmd.id
                        && &e.extension_name == ext_name
                })
                .is_none_or(|e| e.enabled);
            state.tool_registry.push(McpToolDescriptor {
                name: tool.name.clone(),
                description: cmd.description.clone(),
                input_schema: derived_input_schema(tool.input_schema, &cmd.args),
                output_schema: None,
                // A command's own category is a CLI grouping, not a role.
                category: Some(Category::Core.as_str().into()),
                source: Some(ext_name.clone()),
                annotations: None,
            });
            state.surface_entries.push(SurfaceRegistryEntry {
                surface_type: SurfaceType::AutoPromotedTool,
                contribution_name: tool.name,
                extension_name: ext_name.clone(),
                export_name: cmd.export.clone(),
                enabled,
            });
            promoted_count += 1;
        }
    }
    if any_commands {
        state.push_event(
            "commands_auto_promoted",
            json!({"promotedCount": promoted_count, "conflictCount": conflict_count}),
        );
    }
}

/// An extension tool's role: the category it declares when that is one of
/// the four, else `core`. Where it comes from is its `source`.
fn extension_category(declared: Option<&str>) -> &'static str {
    declared
        .and_then(Category::parse)
        .unwrap_or(Category::Core)
        .as_str()
}

/// The manifest spelling of a command arg type.
fn arg_type_name(arg_type: &CommandArgType) -> &'static str {
    match arg_type {
        CommandArgType::String => "string",
        CommandArgType::Path => "path",
        CommandArgType::Bool => "bool",
        CommandArgType::Enum { .. } => "enum",
        CommandArgType::Integer => "integer",
    }
}

/// An extension's short name for tool naming: its manifest `ext_short`,
/// else the last segment of its name (`@specforge/product` -> `product`).
fn ext_short(state: &McpState, ext_name: &str) -> String {
    state
        .manifests
        .iter()
        .find(|m| m.name == ext_name)
        .and_then(|m| m.ext_short.clone())
        .unwrap_or_else(|| {
            ext_name
                .rsplit('/')
                .next()
                .unwrap_or(ext_name)
                .trim_start_matches('@')
                .to_string()
        })
}

/// Complete the per-arg types of `schema` with what the args also declare:
/// enum values, descriptions, and which args are required.
fn derived_input_schema(mut schema: Value, args: &[CommandArg]) -> Value {
    for arg in args {
        let Some(property) = schema["properties"].get_mut(&arg.name) else {
            continue;
        };
        if let CommandArgType::Enum { values } = &arg.arg_type {
            property["enum"] = json!(values);
        }
        if let Some(description) = &arg.description {
            property["description"] = json!(description);
        }
    }
    let required: Vec<&str> = args
        .iter()
        .filter(|a| a.required)
        .map(|a| a.name.as_str())
        .collect();
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

pub fn handle_list_tools(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    // outputSchema came with structuredContent, in 2025-06-18.
    let structured = state.sends_structured_content();
    let tools: Vec<Value> = state
        .tool_registry
        .iter()
        .filter(|t| {
            !disabled(
                state,
                &t.name,
                &[SurfaceType::McpTool, SurfaceType::AutoPromotedTool],
            )
        })
        .map(|t| {
            let mut tool = serde_json::to_value(t).unwrap();
            if !structured && let Some(listed) = tool.as_object_mut() {
                listed.remove("outputSchema");
            }
            tool
        })
        .collect();
    push_discovery(state, "tools", tools.len());
    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

pub fn handle_list_resources(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    let resources: Vec<Value> = listed_resources(state)
        .filter(|r| !is_template(r))
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
    push_discovery(state, "resources", resources.len());
    JsonRpcResponse::success(id, json!({ "resources": resources }))
}

/// MCP `resources/templates/list`: the resources whose URI is a template,
/// as `ResourceTemplate`s.
pub fn handle_list_resource_templates(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    let templates: Vec<Value> = listed_resources(state)
        .filter(|r| is_template(r))
        .map(|r| {
            let mut template = json!({ "uriTemplate": r.uri, "name": r.name });
            if let Some(description) = &r.description {
                template["description"] = json!(description);
            }
            if let Some(mime_type) = &r.mime_type {
                template["mimeType"] = json!(mime_type);
            }
            template
        })
        .collect();
    push_discovery(state, "resource_templates", templates.len());
    JsonRpcResponse::success(id, json!({ "resourceTemplates": templates }))
}

/// The registered resources a listing advertises: all but disabled
/// extension contributions.
fn listed_resources(state: &McpState) -> impl Iterator<Item = &McpResourceDescriptor> {
    state
        .resource_registry
        .iter()
        .filter(|r| !disabled(state, &r.name, &[SurfaceType::McpResource]))
}

/// Whether a resource's URI is an RFC 6570 template (`{placeholder}`).
fn is_template(resource: &McpResourceDescriptor) -> bool {
    resource.uri.contains('{')
}

/// Whether an extension contributed `name` as one of `types` and that
/// contribution is disabled: disabled contributions are not advertised.
fn disabled(state: &McpState, name: &str, types: &[SurfaceType]) -> bool {
    state
        .surface_entries
        .iter()
        .any(|e| !e.enabled && e.contribution_name == name && types.contains(&e.surface_type))
}

pub fn handle_list_prompts(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    let prompts: Vec<Value> = state
        .prompt_registry
        .iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();
    push_discovery(state, "prompts", prompts.len());
    JsonRpcResponse::success(id, json!({ "prompts": prompts }))
}

/// Record a listing: which registry, and how many entries the client got.
fn push_discovery(state: &mut McpState, discovery_type: &str, result_count: usize) {
    state.push_event(
        "mcp_discovery_invoked",
        json!({"discoveryType": discovery_type, "resultCount": result_count}),
    );
}

/// How many tools the server registers before any extension surface.
pub fn default_tool_count() -> usize {
    default_tools().len()
}

/// How many resources the server registers before any extension surface.
pub fn default_resource_count() -> usize {
    default_resources().len()
}

fn default_resources() -> Vec<McpResourceDescriptor> {
    crate::resources::CORE_RESOURCES
        .iter()
        .map(crate::resources::ResourceSpec::descriptor)
        .collect()
}

pub fn default_tools() -> Vec<McpToolDescriptor> {
    crate::tools::CORE_TOOLS
        .iter()
        .map(crate::tool::ToolSpec::descriptor)
        .collect()
}

fn default_prompts() -> Vec<McpPromptDescriptor> {
    crate::prompts::CORE_PROMPTS
        .iter()
        .map(crate::prompts::PromptSpec::descriptor)
        .collect()
}
