//! An extension's MCP prompts (ADR 0291, protocol 1.10.0): the descriptor a `surfaces` item declares, and what the
//! host sends the prompt export and what it answers.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::GraphWire;

/// An MCP prompt an extension contributes: served as `specforge.<short>.<id>`, rendered by `export`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptDescriptor {
    /// snake_case, as a command id; its route is `(short, id with '_' spelled '-')`, shared with commands.
    pub id: String,
    pub title: String,
    pub description: String,
    /// `prompt__<id>` (the SDK derives it).
    pub export: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<PromptArgDescriptor>,
    /// Local codes of the extension. The prompt is refused while one fails (the entry gate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<String>,
    /// The arg (a `reference`) whose entity scopes the gates; absent, the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// The steps the agent takes, in order: rendered with each one's gate state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<PromptStep>,
    /// The export receives the graph, as a command does. Absent, a workflow reads through tools.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reads_graph: bool,
    /// The export receives the extension's state (protocol 1.10.0, ADR 0292).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reads_state: bool,
    /// A bool setting of the extension: while it is false the prompt is not served.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<Attribution>,
}

/// A typed prompt argument: a field's type and bounds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptArgDescriptor {
    pub name: String,
    /// `string`, `integer`, `bool`, `enum`, `string_list`, `enum_list`, `reference`, `reference_list`, `date`, `path`.
    pub field_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    /// A reference's kind, or kinds as `a|b`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One step of a workflow. At most one of `command`, `agent`, `prompt`; none is a step the agent does itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptStep {
    pub id: String,
    pub title: String,
    /// A command id of this extension, or `<peer>:<id>` of a declared peer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// An agent id of the extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// A sub-prompt id of this extension or a peer's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Where ported text came from. Rendered with the prompt and into every derived host file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attribution {
    pub upstream: String,
    pub path: String,
    pub version: String,
    /// An SPDX identifier.
    pub license: String,
    pub copyright: String,
}

/// What the host sends a `prompt__*` export. `G` is how the graph is held, as in [`crate::CommandInput`]: the host
/// sends a `RawGraph`, the SDK reads its indexed graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PromptInput<G = GraphWire> {
    /// Typed JSON: a reference as its full path.
    #[serde(default)]
    pub args: Map<String, Value>,
    pub cwd: String,
    pub today: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<G>,
    /// The host the prompt is rendered for (`claude-code`, `codex`, `cursor`), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// A prompt that declares `reads_state`: the extension's state, read only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<crate::StateView>,
}

/// What a `prompt__*` export answers: at least one message.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptOutput {
    pub messages: Vec<PromptMessage>,
}

/// One message of a rendered prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub text: String,
}

/// Who a prompt message speaks as.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    #[default]
    User,
    Assistant,
}

impl PromptOutput {
    /// A prompt that answers one user message.
    pub fn user(text: impl Into<String>) -> Self {
        PromptOutput {
            messages: vec![PromptMessage {
                role: PromptRole::User,
                text: text.into(),
            }],
        }
    }
}
