use serde_json::{json, Value};
use specforge_registry::SurfaceContributions;

use crate::state::McpState;
use crate::protocol::JsonRpcResponse;
use crate::types::{McpToolDescriptor, McpResourceDescriptor, McpPromptDescriptor, McpPromptArgument};

pub fn register_defaults(state: &mut McpState) {
    state.resource_registry = default_resources();
    state.tool_registry = default_tools();
    state.prompt_registry = default_prompts();
}

/// Convert manifest surface contributions into MCP tool and resource descriptors,
/// appending them to the existing registries.
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
}

pub fn handle_list_tools(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    state.push_event("mcp_discovery_invoked", json!({"kind": "tools"}));
    let tools: Vec<Value> = state.tool_registry.iter()
        .map(|t| serde_json::to_value(t).unwrap())
        .collect();
    JsonRpcResponse::success(id, json!({ "tools": tools }))
}

pub fn handle_list_resources(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    state.push_event("mcp_discovery_invoked", json!({"kind": "resources"}));
    let resources: Vec<Value> = state.resource_registry.iter()
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
    JsonRpcResponse::success(id, json!({ "resources": resources }))
}

pub fn handle_list_prompts(state: &mut McpState, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, -32600, "Server not initialized");
    }
    state.push_event("mcp_discovery_invoked", json!({"kind": "prompts"}));
    let prompts: Vec<Value> = state.prompt_registry.iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();
    JsonRpcResponse::success(id, json!({ "prompts": prompts }))
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

fn default_tools() -> Vec<McpToolDescriptor> {
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
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "Filter by entity kinds" }
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
                    "scope": { "type": "string", "description": "Scope to entity subgraph" }
                }
            }),
            category: Some("core".into()),
        },
        McpToolDescriptor {
            name: "specforge.trace".into(),
            description: "Show traceability chain for an entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Entity ID to trace" },
                    "plan": { "type": "object", "description": "Optional plan JSON for gap detection" }
                },
                "required": ["entity_id"]
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
                    "limit": { "type": "integer", "description": "Max results (default 20)", "default": 20 }
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
                    "kind": { "type": "string", "description": "Filter schema to a specific entity kind" }
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
            name: "specforge.coverage".into(),
            description: "Get coverage status per entity".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Filter to specific entity" },
                    "kind": { "type": "string", "description": "Filter by entity kind" }
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
                    "entity_id": { "type": "string", "description": "Entity ID (optional, all if omitted)" }
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
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Files to format" },
                    "check": { "type": "boolean", "description": "Check only, don't modify", "default": false }
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
                    "new_name": { "type": "string", "description": "New entity ID" }
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
                    "path": { "type": "string", "description": "Project path" },
                    "name": { "type": "string", "description": "Project name" },
                    "extensions": { "type": "array", "items": { "type": "string" } }
                }
            }),
            category: Some("mutation".into()),
        },
        McpToolDescriptor {
            name: "specforge.add_extension".into(),
            description: "Install an extension".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "specifier": { "type": "string", "description": "Extension specifier" }
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
                    "force": { "type": "boolean", "description": "Force removal", "default": false }
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
                    "from_version": { "type": "string", "description": "Source version" },
                    "to_version": { "type": "string", "description": "Target version" }
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
            description: "Collect test results from a runner".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "collector": { "type": "string", "description": "Collector name" }
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
                    "format": { "type": "string", "description": "Output format" },
                    "scope": { "type": "string", "description": "Scope to entity" }
                }
            }),
            category: Some("management".into()),
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
            ]),
        },
        McpPromptDescriptor {
            name: "specforge://prompts/trace".into(),
            description: "Identify traceability gaps for a plan".into(),
            arguments: Some(vec![
                McpPromptArgument {
                    name: "entity_id".into(),
                    description: "Entity ID to trace".into(),
                    required: true,
                },
                McpPromptArgument {
                    name: "plan".into(),
                    description: "Plan JSON to validate against graph".into(),
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
    ]
}
