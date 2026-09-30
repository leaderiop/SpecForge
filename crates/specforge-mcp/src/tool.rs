//! What a tool call produced, and the one place that turns it into a
//! `tools/call` reply.
//!
//! Handlers return a [`ToolOutcome`]; [`envelope`] alone builds `content`,
//! `isError` and `_meta`. The dispatcher reads events and mutation effects
//! from the typed payload, never from the reply text.

use serde_json::{Value, json};
use specforge_common::Diagnostic;

use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};

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
            "diagnostics": serde_json::to_value(&diagnostics).unwrap_or_default(),
        });
    }
    JsonRpcResponse::success(id, result)
}
