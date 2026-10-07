//! What a prompt rendered, and the one place that turns it into a
//! `prompts/get` reply: the prompt-side twin of [`crate::tool`].
//!
//! Each core prompt is one [`PromptSpec`]: its listing is derived from the
//! typed arguments its renderer reads ([`crate::args::Arguments`]), and its renderer
//! returns a [`PromptOutcome`]. [`prompt_envelope`] alone builds the
//! `messages` of a rendered prompt and the error of a refused one (ADR
//! 0004 D4-d).

use serde_json::{Value, json};

use crate::args::Argument;
use crate::protocol::JsonRpcResponse;
use crate::target::{Call, TargetSpec};
use crate::tool::McpError;
use crate::types::{McpPromptArgument, McpPromptDescriptor};

/// What a prompt rendered: the instruction, and the graph data it is about.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub instruction: String,
    pub payload: Value,
}

/// What a `prompts/get` produced. A refusal is the spec's `McpError`, boxed
/// as a tool's refusal is (`call.project()?` converts).
pub type PromptOutcome = Result<Rendered, Box<McpError>>;

/// One core prompt: everything the server lists and renders about it.
pub struct PromptSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// Its arguments ([`crate::args::Arguments::declared`] of its `Args`):
    /// the listing (name, description, required) and the names a request
    /// may send.
    pub arguments: fn() -> Vec<Argument>,
    /// Which project it reads: the served one, brought up to date first.
    pub target: TargetSpec,
    /// Reads the prompt's `Args` from the request's `arguments` (refusing
    /// what does not read) and renders.
    pub render: fn(&Call<'_>, Value) -> PromptOutcome,
}

impl PromptSpec {
    /// The refusal of a request that sends a name neither the prompt nor
    /// its target declares ([`crate::args::undeclared`]).
    pub fn undeclared(&self, arguments: &Value) -> Option<McpError> {
        crate::args::undeclared(arguments, &(self.arguments)(), self.target)
    }

    /// The prompt as `prompts/list` describes it.
    pub fn descriptor(&self) -> McpPromptDescriptor {
        McpPromptDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            arguments: Some(
                (self.arguments)()
                    .into_iter()
                    .map(|argument| McpPromptArgument {
                        name: argument.name.into(),
                        description: argument.description.into(),
                        required: argument.required,
                    })
                    .collect(),
            ),
        }
    }
}

/// The `prompts/get` reply for `outcome`: the only place that builds a
/// prompt's `messages` or a prompt's error. A rendered prompt is its
/// description and two user messages: the instruction, then the payload
/// as JSON text. MCP prompts have no `isError`,
/// so a refusal is a JSON-RPC error under the one code rule
/// ([`McpError::into_rpc_error`]), its `McpError`, naming the prompt, as the
/// error's `data`.
pub fn prompt_envelope(
    outcome: PromptOutcome,
    spec: &PromptSpec,
    id: Option<Value>,
) -> JsonRpcResponse {
    match outcome {
        // Both user messages (C9-14): graph data, user-authored text
        // included, never poses as the model's own earlier turn.
        Ok(Rendered {
            instruction,
            payload,
        }) => JsonRpcResponse::success(
            id,
            json!({
                "description": spec.description,
                "messages": [
                    { "role": "user", "content": { "type": "text", "text": instruction } },
                    { "role": "user", "content": { "type": "text", "text": payload.to_string() } },
                ],
            }),
        ),
        Err(mut error) => {
            error.prompt.get_or_insert_with(|| spec.name.to_string());
            JsonRpcResponse::from_error(id, error.into_rpc_error())
        }
    }
}
