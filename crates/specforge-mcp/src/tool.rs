//! What a tool call produced, and the one place that turns it into a
//! `tools/call` reply.
//!
//! Handlers return a [`ToolOutcome`]; [`envelope`] alone builds `content`,
//! `isError` and `_meta`. The dispatcher reads events and mutation effects
//! from the typed payload, never from the reply text.

use serde_json::{Value, json};
use specforge_common::Diagnostic;

use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::types::McpToolDescriptor;

/// A tool's role (the spec's `McpToolCategory`), plus the `inference`
/// group the listing still shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Core,
    Navigation,
    Mutation,
    Management,
    Inference,
}

impl Category {
    /// The category as `tools/list` spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Core => "core",
            Category::Navigation => "navigation",
            Category::Mutation => "mutation",
            Category::Management => "management",
            Category::Inference => "inference",
        }
    }
}

/// What a completed mutation changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Effect {
    pub files_changed: usize,
    pub entities_affected: usize,
}

/// How a tool that changes files reports it.
#[derive(Debug, Clone, Copy)]
pub struct MutationSpec {
    /// Whether a call with these arguments writes (false for a dry run or a
    /// check).
    pub writes: fn(&Value) -> bool,
    /// What a successful call changed, read from its structured payload.
    pub effect: fn(&Value) -> Effect,
    /// Whether the server recompiles after the call writes: false for a
    /// tool that writes no spec source.
    pub recompiles: bool,
}

/// Every write call unless it is a `dry_run`.
pub fn writes_unless_dry_run(args: &Value) -> bool {
    !args
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// One core tool: everything the server lists, dispatches and reports
/// about it.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub schema: fn() -> Value,
    /// The fields of the handler's `Args` struct ([`crate::args::fields`]):
    /// the arguments it reads.
    pub fields: fn() -> &'static [&'static str],
    pub mutation: Option<MutationSpec>,
    /// The handler, reading its `Args` from the call's `arguments`.
    pub call: fn(&mut McpState, Value) -> ToolOutcome,
}

impl ToolSpec {
    /// The tool as `tools/list` describes it.
    pub fn descriptor(&self) -> McpToolDescriptor {
        McpToolDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            input_schema: (self.schema)(),
            category: Some(self.category.as_str().into()),
        }
    }

    /// The `McpToolCategory` its `mcp_tool_invoked` events carry: a tool
    /// that writes is a mutation; the inference group is core.
    pub fn event_category(&self) -> &'static str {
        if self.mutation.is_some() {
            return Category::Mutation.as_str();
        }
        match self.category {
            Category::Inference => Category::Core.as_str(),
            category => category.as_str(),
        }
    }
}

/// A tool's result body.
#[derive(Debug, Clone)]
pub enum Payload {
    /// A structured result, sent as its JSON text.
    Json(Value),
    /// Plain text blocks, sent as they are: a message, a rendered document,
    /// a command's stdout and stderr.
    Text(Vec<String>),
}

/// What a tool call produced.
#[derive(Debug, Clone)]
pub enum ToolOutcome {
    /// The tool ran. `is_error` marks a failed run (an `isError` result);
    /// `diagnostics` ride in `_meta.diagnostics`; `events` are pushed before
    /// the reply goes out.
    Done {
        payload: Payload,
        is_error: bool,
        diagnostics: Vec<Diagnostic>,
        events: Vec<(String, Value)>,
    },
    /// The call was refused with a JSON-RPC error.
    Refused(JsonRpcError),
}

impl ToolOutcome {
    fn done(payload: Payload, is_error: bool) -> Self {
        ToolOutcome::Done {
            payload,
            is_error,
            diagnostics: Vec::new(),
            events: Vec::new(),
        }
    }

    /// A successful structured result.
    pub fn ok(payload: Value) -> Self {
        Self::done(Payload::Json(payload), false)
    }

    /// A successful plain-text result.
    pub fn text(text: impl Into<String>) -> Self {
        Self::done(Payload::Text(vec![text.into()]), false)
    }

    /// Plain-text blocks, failed or not.
    pub fn texts(blocks: Vec<String>, is_error: bool) -> Self {
        Self::done(Payload::Text(blocks), is_error)
    }

    /// A failed run whose result is a message.
    pub fn failed(message: impl Into<String>) -> Self {
        Self::done(Payload::Text(vec![message.into()]), true)
    }

    /// A failed run whose result is structured.
    pub fn failed_with(payload: Value) -> Self {
        Self::done(Payload::Json(payload), true)
    }

    /// A refusal with the JSON-RPC error `code`.
    pub fn refused(code: i64, message: impl Into<String>) -> Self {
        ToolOutcome::Refused(JsonRpcError::new(code, message))
    }

    /// A refusal whose JSON-RPC error carries `data`.
    pub fn refused_with_data(code: i64, message: impl Into<String>, data: Value) -> Self {
        ToolOutcome::Refused(JsonRpcError::new(code, message).with_data(data))
    }

    /// An invalid-params refusal (`-32602`).
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::refused(error_codes::INVALID_PARAMS, message)
    }

    /// The same outcome, failed when `is_error` (a run whose findings
    /// include errors).
    pub fn flagged(mut self, flag: bool) -> Self {
        if let ToolOutcome::Done { is_error, .. } = &mut self {
            *is_error = flag;
        }
        self
    }

    /// The same outcome with `extra` added to its `_meta.diagnostics`.
    pub fn with_diagnostics(mut self, extra: Vec<Diagnostic>) -> Self {
        if let ToolOutcome::Done { diagnostics, .. } = &mut self {
            diagnostics.extend(extra);
        }
        self
    }

    /// The same outcome with an event to push when it is delivered.
    pub fn with_event(mut self, name: impl Into<String>, params: Value) -> Self {
        if let ToolOutcome::Done { events, .. } = &mut self {
            events.push((name.into(), params));
        }
        self
    }

    /// The structured payload of a successful run.
    pub fn success_payload(&self) -> Option<&Value> {
        match self {
            ToolOutcome::Done {
                payload: Payload::Json(value),
                is_error: false,
                ..
            } => Some(value),
            _ => None,
        }
    }

    /// Whether the tool ran without failing.
    pub fn succeeded(&self) -> bool {
        matches!(
            self,
            ToolOutcome::Done {
                is_error: false,
                ..
            }
        )
    }

    /// Whether the call was refused before the tool ran.
    pub fn is_refused(&self) -> bool {
        matches!(self, ToolOutcome::Refused(_))
    }

    /// Take the events to push, leaving none.
    pub fn take_events(&mut self) -> Vec<(String, Value)> {
        match self {
            ToolOutcome::Done { events, .. } => std::mem::take(events),
            ToolOutcome::Refused(_) => Vec::new(),
        }
    }
}

/// The `tools/call` reply for `outcome`: the only place that builds
/// `content`, `structuredContent`, `isError` and `_meta`. A refusal is a
/// JSON-RPC error. With `structured` (a 2025-06-18 or later session), a
/// JSON object payload is also sent as `structuredContent`, beside the
/// text block holding its JSON.
pub fn envelope(outcome: ToolOutcome, id: Option<Value>, structured: bool) -> JsonRpcResponse {
    let (payload, is_error, diagnostics) = match outcome {
        ToolOutcome::Refused(error) => return JsonRpcResponse::from_error(id, error),
        ToolOutcome::Done {
            payload,
            is_error,
            diagnostics,
            ..
        } => (payload, is_error, diagnostics),
    };
    let content: Vec<Value> = match &payload {
        Payload::Json(value) => vec![json!({ "type": "text", "text": value.to_string() })],
        Payload::Text(blocks) => blocks
            .iter()
            .map(|text| json!({ "type": "text", "text": text }))
            .collect(),
    };
    let mut result = json!({ "content": content, "isError": is_error });
    if let Payload::Json(object @ Value::Object(_)) = payload
        && structured
    {
        result["structuredContent"] = object;
    }
    if !diagnostics.is_empty() {
        result["_meta"] = json!({
            "diagnostics": serde_json::to_value(specforge_emitter::diagnostics_json(&diagnostics))
                .unwrap_or_default(),
        });
    }
    JsonRpcResponse::success(id, result)
}
