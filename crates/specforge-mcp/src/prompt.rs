//! What a prompt rendered, and the one place that turns it into a
//! `prompts/get` reply: the prompt-side twin of [`crate::tool`].
//!
//! Each core prompt is one [`PromptSpec`]: its listing is derived from the
//! typed arguments its renderer reads ([`arguments`]), and its renderer
//! returns a [`PromptOutcome`]. [`prompt_envelope`] alone builds the
//! `messages` of a rendered prompt and the error of a refused one (ADR
//! 0004 D4-d).

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

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

/// A prompt's typed arguments. The listing is derived from the type, so it
/// cannot drift from what the renderer reads.
pub trait PromptArgs: DeserializeOwned {
    /// Each argument's description, keyed by field name. A test holds the
    /// keys to exactly the struct's fields.
    const DESCRIPTIONS: &'static [(&'static str, &'static str)];
}

/// One core prompt: everything the server lists and renders about it.
pub struct PromptSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// The listing's arguments ([`arguments`] of the prompt's `Args`).
    pub arguments: fn() -> Vec<McpPromptArgument>,
    /// The fields of the prompt's `Args` ([`crate::args::fields`]): the
    /// arguments the renderer reads.
    pub fields: fn() -> &'static [&'static str],
    /// The prompt's [`PromptArgs::DESCRIPTIONS`], for the drift test.
    pub descriptions: &'static [(&'static str, &'static str)],
    /// Which project it reads: the served one, brought up to date first.
    pub target: TargetSpec,
    /// Reads the prompt's `Args` from the request's `arguments` (refusing
    /// what does not parse) and renders.
    pub render: fn(&Call<'_>, Value) -> PromptOutcome,
}

impl PromptSpec {
    /// The refusal of a request that sends a name neither the prompt nor
    /// its target declares ([`crate::args::unknown_argument`]).
    pub fn undeclared(&self, arguments: &Value) -> Option<McpError> {
        let known: Vec<&str> = (self.fields)()
            .iter()
            .chain(self.target.accepted())
            .copied()
            .collect();
        crate::args::unknown_argument(arguments, &known)
    }

    /// The prompt as `prompts/list` describes it.
    pub fn descriptor(&self) -> McpPromptDescriptor {
        McpPromptDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            arguments: Some((self.arguments)()),
        }
    }
}

/// The listing of `A`'s arguments: names from the serde field tracer, in
/// field order; `required` exactly for the fields `A` cannot be read
/// without; descriptions from [`PromptArgs::DESCRIPTIONS`].
pub fn arguments<A: PromptArgs>() -> Vec<McpPromptArgument> {
    let required = crate::args::required::<A>();
    crate::args::fields::<A>()
        .iter()
        .map(|name| McpPromptArgument {
            name: (*name).into(),
            description: A::DESCRIPTIONS
                .iter()
                .find(|(field, _)| field == name)
                .map_or("", |(_, text)| text)
                .into(),
            required: required.contains(name),
        })
        .collect()
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
