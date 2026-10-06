use serde_json::{Value, json};
use specforge_protocol_types::{
    CommandArgDescriptor, CommandArgType, CommandDescriptor, ExtensionDeclaration,
};
use specforge_registry::{SurfaceRegistryEntry, SurfaceType};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use crate::tool::Category;
use crate::types::{McpResourceDescriptor, McpToolDescriptor};

pub fn register_defaults(state: &mut McpState) {
    state.resource_registry = default_resources();
    state.tool_registry = default_tools();
}

/// Convert the declarations' surfaces into MCP tool and resource descriptors,
/// appending them to the existing registries, then auto-promote every CLI
/// command to an MCP tool (see [`auto_promote_commands`]).
pub fn register_extension_surfaces(state: &mut McpState, declarations: &[ExtensionDeclaration]) {
    for declaration in declarations {
        let ext_name = declaration.name();
        let surfaces = &declaration.surfaces;
        for tool in &surfaces.mcp_tools {
            state.tool_registry.push(McpToolDescriptor {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                // The declared output_schema is the tool's outputSchema.
                output_schema: tool.output_schema.clone(),
                category: Some(extension_category(tool.category.as_deref()).into()),
                source: Some(ext_name.to_string()),
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
    auto_promote_commands(state, declarations);
}

/// Every extension CLI command becomes the MCP tool
/// `specforge.{ext_short}.{cmd_id}`, its input schema derived from the
/// command's args, dispatched to the command's export; but a command the
/// host refuses (`specforge_ops::command::refusal`), which no surface runs. A tool already
/// registered under that name (core or explicitly contributed) wins, and
/// the command is reported with I017. Emits `commands_auto_promoted` when
/// any extension contributes commands.
fn auto_promote_commands(state: &mut McpState, declarations: &[ExtensionDeclaration]) {
    let mut promoted_count = 0;
    let mut conflict_count = 0;
    let mut any_commands = false;
    for declaration in declarations {
        let ext_name = declaration.name();
        let surfaces = &declaration.surfaces;
        if surfaces.commands.is_empty() {
            continue;
        }
        any_commands = true;
        let explicit: std::collections::HashSet<String> =
            state.tool_registry.iter().map(|t| t.name.clone()).collect();
        // A command the host refuses (an arg taking a host option, such as
        // `format`) is no tool, as it is no command line.
        let promotable: Vec<&CommandDescriptor> = surfaces
            .commands
            .iter()
            .filter(|cmd| specforge_ops::command::refusal(cmd).is_none())
            .collect();
        let args: Vec<Vec<(&str, &str)>> = promotable
            .iter()
            .map(|cmd| {
                cmd.args
                    .iter()
                    .map(|arg| (arg.name.as_str(), arg_type_name(&arg.arg_type)))
                    .collect()
            })
            .collect();
        let commands: Vec<(&str, &[(&str, &str)])> = promotable
            .iter()
            .zip(&args)
            .map(|(cmd, args)| (cmd.id.as_str(), args.as_slice()))
            .collect();
        let short = declaration.short();
        let (tools, diagnostics) =
            specforge_wasm::auto_promote_commands_to_mcp_tools(&commands, &explicit, &short);
        conflict_count += diagnostics.len();
        state.surface_diagnostics.extend(diagnostics);

        for tool in tools {
            let Some(cmd) = promotable.iter().find(|c| c.id == tool.source_command_id) else {
                continue;
            };
            state.tool_registry.push(McpToolDescriptor {
                name: tool.name.clone(),
                description: cmd.description.clone(),
                input_schema: derived_input_schema(tool.input_schema, &cmd.args),
                output_schema: None,
                // A command's own category is a CLI grouping, not a role.
                category: Some(Category::Core.as_str().into()),
                source: Some(ext_name.to_string()),
                annotations: None,
            });
            state.promoted_surfaces.push(SurfaceRegistryEntry {
                surface_type: SurfaceType::AutoPromotedTool,
                contribution_name: tool.name,
                extension_name: ext_name.to_string(),
                export_name: cmd.export.clone(),
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

/// The declared spelling of a command arg type.
fn arg_type_name(arg_type: &CommandArgType) -> &'static str {
    match arg_type {
        CommandArgType::String => "string",
        CommandArgType::Path => "path",
        CommandArgType::Bool => "bool",
        CommandArgType::Enum { .. } => "enum",
        CommandArgType::Integer => "integer",
    }
}

/// Complete the per-arg types of `schema` with what the args also declare:
/// enum values, descriptions, and which args are required.
fn derived_input_schema(mut schema: Value, args: &[CommandArgDescriptor]) -> Value {
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
    let resources: Vec<Value> = state
        .resource_registry
        .iter()
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
    let templates: Vec<Value> = state
        .resource_registry
        .iter()
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

/// Whether a resource's URI is an RFC 6570 template (`{placeholder}`).
fn is_template(resource: &McpResourceDescriptor) -> bool {
    resource.uri.contains('{')
}

pub fn handle_list_prompts(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    // The core prompts, derived from their table: no extension declares a
    // prompt.
    let prompts: Vec<Value> = crate::prompts::descriptors()
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
