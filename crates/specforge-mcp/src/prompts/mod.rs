mod context;
mod explore;
mod infer;
mod review;
mod trace;

use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::types::{McpPromptArgument, McpPromptDescriptor};

/// One argument a prompt takes.
pub struct PromptArg {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

/// One core prompt: everything the server lists and renders about it.
pub struct PromptSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub arguments: &'static [PromptArg],
    pub(crate) get: fn(&McpState, Value, Option<Value>) -> JsonRpcResponse,
}

impl PromptSpec {
    /// The prompt as `prompts/list` describes it.
    pub fn descriptor(&self) -> McpPromptDescriptor {
        McpPromptDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            arguments: Some(
                self.arguments
                    .iter()
                    .map(|a| McpPromptArgument {
                        name: a.name.into(),
                        description: a.description.into(),
                        required: a.required,
                    })
                    .collect(),
            ),
        }
    }
}

/// The core prompts, in listing order.
pub static CORE_PROMPTS: &[PromptSpec] = &[
    PromptSpec {
        name: "specforge://prompts/context",
        description: "Get structured context for implementing an entity",
        arguments: &[
            PromptArg {
                name: "entity_id",
                description: "Entity ID to get context for",
                required: true,
            },
            PromptArg {
                name: "structural_constraints",
                description: "Entity IDs to include as context even when not connected (array or comma-separated)",
                required: false,
            },
        ],
        get: context::get,
    },
    PromptSpec {
        name: "specforge://prompts/review",
        description: "Analyze coverage gaps for an entity or the whole graph",
        arguments: &[
            PromptArg {
                name: "entity_id",
                description: "Entity ID to review (optional, reviews all if omitted)",
                required: false,
            },
            PromptArg {
                name: "depth",
                description: "Neighbor hops around entity_id to include (default 1)",
                required: false,
            },
        ],
        get: review::get,
    },
    PromptSpec {
        name: "specforge://prompts/trace",
        description: "Identify traceability gaps for a plan",
        arguments: &[
            PromptArg {
                name: "plan",
                description: "AgentPlan JSON ({\"entries\": [{\"entity_id\", \"action\"}]}) to check against the graph",
                required: false,
            },
            PromptArg {
                name: "entity_id",
                description: "Entity ID to trace when no plan is given",
                required: false,
            },
        ],
        get: trace::get,
    },
    PromptSpec {
        name: "specforge://prompts/explore",
        description: "Discover exploration starting points in the graph",
        arguments: &[
            PromptArg {
                name: "entity_id",
                description: "Starting entity (optional)",
                required: false,
            },
            PromptArg {
                name: "kind",
                description: "Filter by entity kind",
                required: false,
            },
        ],
        get: explore::get,
    },
    PromptSpec {
        name: "specforge://prompts/infer",
        description: "Get inference guidance for discovering spec entities from code",
        arguments: &[
            PromptArg {
                name: "scope",
                description: "Scope: omit for overview, 'kind:{name}' for focused guide, 'file:{path}' for file deduplication",
                required: false,
            },
            PromptArg {
                name: "target_spec_directory",
                description: "Directory where generated .spec files are written (scope \"plan\")",
                required: false,
            },
            PromptArg {
                name: "cursor",
                description: "Offset into the plan's unanalyzed/stale file lists for paging (scope \"plan\")",
                required: false,
            },
        ],
        get: infer::get,
    },
];

pub fn handle_prompt_get(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: name",
            );
        }
    };

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let mut event = serde_json::json!({ "promptName": name });
    for (argument, field) in [("entity_id", "entityId"), ("kind", "kind")] {
        if let Some(value) = arguments.get(argument).and_then(Value::as_str) {
            event[field] = Value::from(value);
        }
    }
    state.push_event("mcp_prompt_invoked", event);

    match CORE_PROMPTS.iter().find(|p| p.name == name) {
        Some(prompt) => (prompt.get)(state, arguments, id),
        None => JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Unknown prompt: {}", name),
        ),
    }
}
