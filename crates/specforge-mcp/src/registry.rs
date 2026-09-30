use serde_json::{Value, json};
use specforge_registry::{
    CommandArg, CommandArgType, SurfaceContributions, SurfaceRegistryEntry, SurfaceType,
};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use crate::types::{
    McpPromptArgument, McpPromptDescriptor, McpResourceDescriptor, McpToolDescriptor,
};

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
    for (_ext_name, surfaces) in manifest_surfaces {
        for tool in &surfaces.mcp_tools {
            state.tool_registry.push(McpToolDescriptor {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                category: tool.category.clone().or_else(|| Some("extension".into())),
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
                category: Some("extension".into()),
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

/// The manifest spelling of a command arg type.
fn arg_type_name(arg_type: &CommandArgType) -> &'static str {
    match arg_type {
        CommandArgType::StringArg => "string",
        CommandArgType::PathArg => "path",
        CommandArgType::BoolArg => "bool",
        CommandArgType::EnumArg { .. } => "enum",
        CommandArgType::IntegerArg => "integer",
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
        if let CommandArgType::EnumArg { values } = &arg.arg_type {
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
        .map(|t| serde_json::to_value(t).unwrap())
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
        .filter(|r| !disabled(state, &r.name, &[SurfaceType::McpResource]))
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
    push_discovery(state, "resources", resources.len());
    JsonRpcResponse::success(id, json!({ "resources": resources }))
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
    vec![
        McpResourceDescriptor {
            uri: "specforge://graph".into(),
            name: "graph".into(),
            description: Some("Full spec graph in JSON format".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://schema".into(),
            name: "schema".into(),
            description: Some("Graph schema definition".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://context".into(),
            name: "context".into(),
            description: Some("Context-optimized graph (contract, status, verify fields)".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://context/{entity_id}".into(),
            name: "context_entity".into(),
            description: Some("Context-optimized subgraph rooted at an entity".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://brief".into(),
            name: "brief".into(),
            description: Some("Brief graph (id, kind, title, edges only)".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://diagnostics".into(),
            name: "diagnostics".into(),
            description: Some("Current compilation diagnostics".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://graph/{entity_id}".into(),
            name: "entity".into(),
            description: Some("Subgraph rooted at a specific entity".into()),
            mime_type: Some("application/json".into()),
        },
        McpResourceDescriptor {
            uri: "specforge://entities/{kind}".into(),
            name: "entities_by_kind".into(),
            description: Some("All entities of a specific kind (e.g. feature, behavior)".into()),
            mime_type: Some("application/json".into()),
        },
    ]
}

pub fn default_tools() -> Vec<McpToolDescriptor> {
    vec![
        // Core tools
        McpToolDescriptor {
            name: "specforge.query".into(),
            description: "Query the graph at multiple resolutions".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to query" },
                    "depth": { "type": "integer", "description": "Number of hops (default 1)", "default": 1 },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "Filter by entity kinds" },
                    "format": { "type": "string", "description": "Output detail level (default \"graph\")", "default": "graph" },
                    "include_coverage": { "type": "boolean", "description": "Include coverage metadata in the response", "default": false }
                },
                "required": ["entity_id"]
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.validate".into(),
            description: "Recompile and validate the spec project".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" },
                    "severity_filter": { "type": "string", "description": "Only report diagnostics of this severity (error, warning, info)" },
                    "strict": { "type": "boolean", "description": "Promote warnings to errors, before severity_filter applies", "default": false },
                    "use_cached": { "type": "boolean", "description": "Report cached diagnostics from the last compile instead of recompiling", "default": false }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.analyze".into(),
            description: "Run analysis passes (coverage: proof obligations and discharge funnel; contracts: clause symmetry) over the compiled project".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pass": { "type": "string", "description": "Analysis pass to run (all, coverage, contracts)" },
                    "strict": { "type": "boolean", "description": "Promote warnings to errors" },
                    "test_results": { "type": "string", "description": "Path to a specforge-report.json for proof-level verdicts" },
                    "use_cached": { "type": "boolean", "description": "Analyze the last compiled graph instead of recompiling (a server with no graph compiles anyway)", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.export".into(),
            description: "Export the graph in various formats".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "format": { "type": "string", "enum": ["graph", "context", "brief"], "default": "graph" },
                    "scope": { "type": "string", "description": "Scope to entity subgraph" },
                    "max_tokens": { "type": "integer", "description": "Optional token budget; truncates the export to the most central entities that fit" }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.trace".into(),
            description: "Show traceability chain for an entity, or check an agent plan for gaps (entity_id or plan)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to trace" },
                    "plan": { "type": "object", "description": "An agent plan, {\"entries\": [{\"entity_id\": ...}]}, to check for gaps against the graph instead of tracing one entity" }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.search".into(),
            description: "Fuzzy search over graph nodes".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "Filter by kinds" },
                    "limit": { "type": "integer", "description": "Max results (default 20)", "default": 20 },
                    "field": { "type": "string", "description": "Only search a specific field" },
                    "value": { "type": "string", "description": "Exact field value filter (with field)" },
                    "references": { "type": "string", "description": "Find entities with edges to this target" }
                },
                "required": ["query"]
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.schema".into(),
            description: "Get the graph schema definition".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter schema to a specific entity kind" },
                    "include_edges": { "type": "boolean", "description": "Include edge labels", "default": true },
                    "include_validation_rules": { "type": "boolean", "description": "Include the validation rules loaded extensions declare", "default": false }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.model".into(),
            description: "Render the logical data model (entity kinds, fields, relationships)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "mermaid", "dot", "json", "dbml"],
                        "description": "Output format (default: markdown)"
                    },
                    "group_by": {
                        "type": "string",
                        "enum": ["extension", "none"],
                        "description": "Group entities by extension or list flat (default: extension)"
                    },
                    "fields": {
                        "type": "string",
                        "enum": ["none", "keys", "all"],
                        "description": "Field detail level (default: keys)"
                    },
                    "extension": {
                        "type": "string",
                        "description": "Filter to a single extension"
                    },
                    "kinds": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter to specific entity kinds"
                    },
                    "root": {
                        "type": "string",
                        "description": "Root entity kind for depth-scoped output"
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum depth from root kind (requires root)"
                    }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.outline_extensions".into(),
            description: "Renders the extension architecture hierarchy — how extensions relate via dependencies, enhancements, and cross-extension edges. Shows entity kinds, edge types, validation rules, and surface contributions per extension. Use this to understand the project's extension topology before making structural changes.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "format": {
                        "type": "string",
                        "enum": ["markdown", "mermaid", "dot", "json"],
                        "description": "Output format (default: json). JSON recommended for programmatic consumption."
                    },
                    "fields": {
                        "type": "string",
                        "enum": ["none", "keys", "all"],
                        "description": "Detail level: none (counts only), keys (names + rule codes), all (full field attribution). Default: keys"
                    },
                    "deps": {
                        "type": "string",
                        "enum": ["direct", "effective", "full"],
                        "description": "Dependency visibility: direct (declared only), effective (direct + used transitive), full (all transitive). Default: direct"
                    }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.coverage".into(),
            description: "Get coverage status per entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Filter to specific entity" },
                    "kind": { "type": "string", "description": "Filter by entity kind" },
                    "status_filter": { "type": "string", "enum": ["covered", "partial", "uncovered"], "description": "Only entities with this coverage status" }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.stats".into(),
            description: "Get project statistics".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            category: Some("core".into()),
        },
        // Dynamic tools
        McpToolDescriptor {
            name: "specforge.list".into(),
            description: "List entities, optionally filtered by kind".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Filter by entity kind (e.g. 'feature', 'behavior')" }
                }
            }),
            category: Some("core".into()),
        },
        // Navigation tools
        McpToolDescriptor {
            name: "specforge.inspect".into(),
            description: "Get full detail for a specific entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to inspect" }
                },
                "required": ["entity_id"]
            }),
            category: Some("navigation".into()),
        },
        McpToolDescriptor {
            name: "specforge.find_definition".into(),
            description: "Find the source location of an entity definition".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID" }
                },
                "required": ["entity_id"]
            }),
            category: Some("navigation".into()),
        },
        McpToolDescriptor {
            name: "specforge.find_references".into(),
            description: "Find all references to an entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID" }
                },
                "required": ["entity_id"]
            }),
            category: Some("navigation".into()),
        },
        McpToolDescriptor {
            name: "specforge.outline".into(),
            description: "Get entity outline for a file".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file": { "type": "string", "description": "File path" }
                },
                "required": ["file"]
            }),
            category: Some("navigation".into()),
        },
        McpToolDescriptor {
            name: "specforge.suggest_fixes".into(),
            description: "Get suggested fixes for diagnostics".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID (optional, all if omitted)" },
                    "file_path": { "type": "string", "description": "Only diagnostics in this spec file" },
                    "diagnostic_code": { "type": "string", "description": "Only diagnostics with this code, e.g. W001" }
                }
            }),
            category: Some("navigation".into()),
        },
        // Mutation tools
        McpToolDescriptor {
            name: "specforge.format".into(),
            description: "Format spec files".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" },
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Files or directories to format, relative to the project root (defaults to every spec file)" },
                    "check": { "type": "boolean", "description": "Check only, don't modify", "default": false },
                    "diff": { "type": "boolean", "description": "Return a before/after diff for each file that would change, without modifying it", "default": false },
                    "write": { "type": "boolean", "description": "Write formatted output (defaults to false in check or diff mode)", "default": true }
                }
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.rename".into(),
            description: "Rename an entity across all files".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Current entity ID" },
                    "new_name": { "type": "string", "description": "New entity ID" },
                    "dry_run": { "type": "boolean", "description": "Return the rename plan without changing any file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["entity_id", "new_name"]
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.init".into(),
            description: "Initialize a new SpecForge project".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory for the new project, outside the current one" },
                    "name": { "type": "string", "description": "Project name (defaults to the directory name)" },
                    "version": { "type": "string", "description": "Project version", "default": "0.1.0" },
                    "extensions": { "type": "array", "items": { "type": "string" }, "description": "Builtin extensions to enable, e.g. @specforge/software" }
                },
                "required": ["path"]
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.add_extension".into(),
            description: "Install an extension".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "specifier": { "type": "string", "description": "Extension specifier" },
                    "dry_run": { "type": "boolean", "description": "Preview the install without changing any file", "default": false },
                    "allow_unsigned": { "type": "boolean", "description": "Accept a registry package with no publisher signature (publisher verification skipped)", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["specifier"]
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.remove_extension".into(),
            description: "Remove an installed extension".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Extension name" },
                    "force": { "type": "boolean", "description": "Force removal", "default": false },
                    "dry_run": { "type": "boolean", "description": "Preview the removal, orphan warnings included, without changing any file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                },
                "required": ["name"]
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.migrate".into(),
            description: "Run migration pipeline".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "dry_run": { "type": "boolean", "description": "Return the diffs without changing any file", "default": false },
                    "target_version": { "type": "string", "description": "Format version to migrate to, as MAJOR.MINOR (defaults to the current format version)" },
                    "no_backup": { "type": "boolean", "description": "Skip the .bak backup of each migrated file", "default": false },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            }),
            category: Some("mutation".into()),
        },
        // Management tools
        McpToolDescriptor {
            name: "specforge.extensions".into(),
            description: "List installed extensions".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
            category: Some("management".into()),
        },
        McpToolDescriptor {
            name: "specforge.providers".into(),
            description: "List configured providers".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
            category: Some("management".into()),
        },
        McpToolDescriptor {
            name: "specforge.doctor".into(),
            description: "Run health checks".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
            category: Some("management".into()),
        },
        McpToolDescriptor {
            name: "specforge.collect".into(),
            description: "Record which entities the project's tests prove, from the test runner's report (runs the runner only with run: true and prior approval)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "runner": { "type": "string", "description": "Collector name (e.g. cargo-test); detected from project files if omitted" },
                    "run": { "type": "boolean", "description": "Run the test command first; it must have been approved with `specforge collect` in a terminal (default false: parse the existing report)" },
                    "path": { "type": "string", "description": "Project root path (uses initialized root if omitted)" }
                }
            }),
            category: Some("management".into()),
        },
        McpToolDescriptor {
            name: "specforge.render".into(),
            description: "Render output in a specified format".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "format": { "type": "string", "enum": ["json", "dot", "context", "brief"], "description": "Renderer to use" },
                    "out_dir": { "type": "string", "description": "Directory to write the rendering into (returned inline when omitted)" },
                    "scope": { "type": "string", "description": "Scope to entity" }
                },
                "required": ["format"]
            }),
            category: Some("management".into()),
        },
        McpToolDescriptor {
            name: "specforge.infer_progress".into(),
            description: "Check inference progress: summary of analyzed vs unanalyzed source files, stale entries, and entity counts".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            category: Some("inference".into()),
        },
        McpToolDescriptor {
            name: "specforge.infer_gaps".into(),
            description: "Analyze inference gaps: public Rust items not yet covered by spec entities (approximate)".into(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
            category: Some("inference".into()),
        },
        McpToolDescriptor {
            name: "specforge.infer_session".into(),
            description: "Manage inference sessions: start a new session, mark files as analyzed, or end a session".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "mark_analyzed", "end"],
                        "description": "Session action to perform"
                    },
                    "agent": {
                        "type": "string",
                        "description": "Agent identifier (for start)"
                    },
                    "source_roots": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Source directories to scan (for start)"
                    },
                    "source_file": {
                        "type": "string",
                        "description": "Relative path to analyzed file (for mark_analyzed)"
                    },
                    "entities_produced": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Entity IDs produced from the file (for mark_analyzed)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID to end (for end)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["completed", "paused"],
                        "description": "Final status (for end, default: completed)"
                    }
                },
                "required": ["action"]
            }),
            category: Some("inference".into()),
        },
        McpToolDescriptor {
            name: "specforge.find_implementation".into(),
            description: "Find source code locations that implement a specforge entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": {
                        "type": "string",
                        "description": "Entity ID to find implementations for"
                    }
                },
                "required": ["entity_id"]
            }),
            category: Some("navigation".into()),
        },
        McpToolDescriptor {
            name: "specforge.find_spec_for_source".into(),
            description: "Find specforge entities anchored to a source file".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Relative path to source file"
                    }
                },
                "required": ["file_path"]
            }),
            category: Some("navigation".into()),
        },
    ]
}

fn default_prompts() -> Vec<McpPromptDescriptor> {
    vec![
        McpPromptDescriptor {
            name: "specforge://prompts/context".into(),
            description: "Get structured context for implementing an entity".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "entity_id".into(),
                    description: "Entity ID to get context for".into(),
                    required: true,
                },
                McpPromptArgument {
                    name: "structural_constraints".into(),
                    description: "Entity IDs to include as context even when not connected (array or comma-separated)".into(),
                    required: false,
                },
            ]),
        },
        McpPromptDescriptor {
            name: "specforge://prompts/review".into(),
            description: "Analyze coverage gaps for an entity or the whole graph".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "entity_id".into(),
                    description: "Entity ID to review (optional, reviews all if omitted)".into(),
                    required: false,
                },
                McpPromptArgument {
                    name: "depth".into(),
                    description: "Neighbor hops around entity_id to include (default 1)".into(),
                    required: false,
                },
            ]),
        },
        McpPromptDescriptor {
            name: "specforge://prompts/trace".into(),
            description: "Identify traceability gaps for a plan".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "plan".into(),
                    description: "AgentPlan JSON ({\"entries\": [{\"entity_id\", \"action\"}]}) to check against the graph".into(),
                    required: false,
                },
                McpPromptArgument {
                    name: "entity_id".into(),
                    description: "Entity ID to trace when no plan is given".into(),
                    required: false,
                },
            ]),
        },
        McpPromptDescriptor {
            name: "specforge://prompts/explore".into(),
            description: "Discover exploration starting points in the graph".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "entity_id".into(),
                    description: "Starting entity (optional)".into(),
                    required: false,
                },
                McpPromptArgument {
                    name: "kind".into(),
                    description: "Filter by entity kind".into(),
                    required: false,
                },
            ]),
        },
        McpPromptDescriptor {
            name: "specforge://prompts/infer".into(),
            description: "Get inference guidance for discovering spec entities from code".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "scope".into(),
                    description: "Scope: omit for overview, 'kind:{name}' for focused guide, 'file:{path}' for file deduplication".into(),
                    required: false,
                },
                McpPromptArgument {
                    name: "target_spec_directory".into(),
                    description: "Directory where generated .spec files are written (scope \"plan\")".into(),
                    required: false,
                },
                McpPromptArgument {
                    name: "cursor".into(),
                    description: "Offset into the plan's unanalyzed/stale file lists for paging (scope \"plan\")".into(),
                    required: false,
                },
            ]),
        },
    ]
}
