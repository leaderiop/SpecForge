//! What a tool call produced, and the one place that turns it into a
//! `tools/call` reply.
//!
//! Handlers return a [`ToolOutcome`]; [`envelope`] alone builds `content`,
//! `isError` and `_meta`. The dispatcher reads events and mutation effects
//! from the typed payload, never from the reply text.

use serde_json::{Value, json};
use specforge_common::Diagnostic;

use crate::protocol::JsonRpcResponse;
use crate::target::{Call, TargetSpec};
use crate::types::McpToolDescriptor;

/// A tool's role: the spec's `McpToolCategory`. Where a tool comes from is
/// its `source`, a separate field (ADR 0004 D4-b).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Core,
    Navigation,
    Mutation,
    Management,
}

impl Category {
    /// The category as `tools/list` spells it.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Core => "core",
            Category::Navigation => "navigation",
            Category::Mutation => "mutation",
            Category::Management => "management",
        }
    }

    /// The category named `name`, if it is one of the four.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "core" => Some(Category::Core),
            "navigation" => Some(Category::Navigation),
            "mutation" => Some(Category::Mutation),
            "management" => Some(Category::Management),
            _ => None,
        }
    }
}

/// The `source` of every core tool; an extension tool's is the
/// extension's name.
pub const CORE_SOURCE: &str = "core";

/// What a tool does to its environment, as MCP's tool annotations say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// It only reads: `readOnlyHint`.
    ReadOnly,
    /// It writes files.
    Writes {
        /// It may overwrite or remove what is there (`destructiveHint`).
        destructive: bool,
        /// Calling it again with the same arguments changes nothing more
        /// (`idempotentHint`).
        idempotent: bool,
        /// It reaches beyond the project: a registry, a test runner
        /// (`openWorldHint`).
        open_world: bool,
    },
}

impl Access {
    /// The MCP `ToolAnnotations` for this access.
    pub fn annotations(self) -> Value {
        match self {
            Access::ReadOnly => json!({ "readOnlyHint": true, "openWorldHint": false }),
            Access::Writes {
                destructive,
                idempotent,
                open_world,
            } => json!({
                "readOnlyHint": false,
                "destructiveHint": destructive,
                "idempotentHint": idempotent,
                "openWorldHint": open_world,
            }),
        }
    }
}

/// What a completed mutation changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Effect {
    pub files_changed: usize,
    pub entities_affected: usize,
}

/// How a tool that changes the project reports it.
#[derive(Debug, Clone, Copy)]
pub struct MutationSpec {
    /// Whether a call with these arguments writes (false for a dry run or a
    /// check).
    pub writes: fn(&Value) -> bool,
    /// What a successful call changed, read from its structured payload.
    pub effect: fn(&Value) -> Effect,
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
    /// What it does to its environment: the listing's annotations.
    pub access: Access,
    pub schema: fn() -> Value,
    /// The schema its `structuredContent` conforms to: for a tool whose
    /// result is a JSON object.
    pub output: Option<fn() -> Value>,
    /// The fields of the handler's `Args` struct ([`crate::args::fields`]):
    /// the arguments it reads.
    pub fields: fn() -> &'static [&'static str],
    /// How a mutation reports what it changed: present exactly for the
    /// `mutation` category.
    pub mutation: Option<MutationSpec>,
    /// Which project it acts on, and whether that project is brought up
    /// to date first: resolved into the call's target before the handler.
    pub target: TargetSpec,
    /// The handler, reading its `Args` from the call's `arguments`.
    pub call: fn(&mut Call<'_>, Value) -> ToolOutcome,
}

impl ToolSpec {
    /// The tool as `tools/list` describes it.
    pub fn descriptor(&self) -> McpToolDescriptor {
        McpToolDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            input_schema: (self.schema)(),
            output_schema: self.output.map(|schema| schema()),
            category: Some(self.category.as_str().into()),
            source: Some(CORE_SOURCE.into()),
            annotations: Some(self.access.annotations()),
        }
    }
}

/// The spec's `McpErrorCode`: the small closed set agents branch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    CompilationFailed,
    EntityNotFound,
    FileNotFound,
    ExtensionNotFound,
    PermissionDenied,
    Timeout,
    NotInitialized,
    SchemaMismatch,
    InternalError,
    Conflict,
    PreconditionFailed,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::InvalidInput => "invalid_input",
            ErrorCode::CompilationFailed => "compilation_failed",
            ErrorCode::EntityNotFound => "entity_not_found",
            ErrorCode::FileNotFound => "file_not_found",
            ErrorCode::ExtensionNotFound => "extension_not_found",
            ErrorCode::PermissionDenied => "permission_denied",
            ErrorCode::Timeout => "timeout",
            ErrorCode::NotInitialized => "not_initialized",
            ErrorCode::SchemaMismatch => "schema_mismatch",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::Conflict => "conflict",
            ErrorCode::PreconditionFailed => "precondition_failed",
        }
    }

    /// The code a failure reported with diagnostic `code` carries.
    pub fn for_diagnostic(code: &str) -> Self {
        match code {
            "E003" => ErrorCode::EntityNotFound,
            "E019" | "E054" | "E064" => ErrorCode::InvalidInput,
            "E027" => ErrorCode::Conflict,
            "E045" => ErrorCode::SchemaMismatch,
            "E058" | "E063" => ErrorCode::PreconditionFailed,
            "E059" => ErrorCode::PermissionDenied,
            "R004" => ErrorCode::Timeout,
            _ => ErrorCode::InternalError,
        }
    }
}

/// A failed tool call: the spec's `McpError`, the content of its `isError`
/// result. The diagnostic code, when there is one, is `diagnostic.code`,
/// never only message text.
#[derive(Debug, Clone)]
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,
    pub tool: Option<String>,
    pub entity_id: Option<String>,
    pub argument: Option<String>,
    pub diagnostic: Option<Value>,
    pub data: Option<Value>,
    /// Other diagnostics the call reported on its way to failing; they
    /// ride in `_meta.diagnostics`, as a successful call's do.
    pub reported: Vec<Diagnostic>,
}

impl McpError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            tool: None,
            entity_id: None,
            argument: None,
            diagnostic: None,
            data: None,
            reported: Vec::new(),
        }
    }

    /// A failure reported as `diagnostic`, its code mapped by
    /// [`ErrorCode::for_diagnostic`].
    pub fn from_diagnostic(diagnostic: &Diagnostic) -> Self {
        Self::new(
            ErrorCode::for_diagnostic(&diagnostic.code),
            diagnostic.message.clone(),
        )
        .with_diagnostic(diagnostic)
    }

    /// A failure whose message leads with a diagnostic code
    /// (`"E003: unresolved entity 'x' …"`): the code moves to `diagnostic`.
    pub fn from_coded_message(fallback: ErrorCode, message: &str) -> Self {
        match split_code(message) {
            Some((code, rest)) => Self::from_diagnostic(&Diagnostic::error(code, rest)),
            None => Self::new(fallback, message),
        }
    }

    pub fn with_entity(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }

    pub fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.argument = Some(argument.into());
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: &Diagnostic) -> Self {
        self.diagnostic = serde_json::to_value(specforge_common::diagnostics_json(
            std::slice::from_ref(diagnostic),
        ))
        .ok()
        .and_then(|mut all| all.get_mut(0).map(Value::take));
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// The error as its `isError` result carries it.
    pub fn to_json(&self) -> Value {
        let mut error = json!({ "code": self.code.as_str(), "message": self.message });
        for (key, value) in [
            ("tool", self.tool.clone().map(Value::from)),
            ("entity_id", self.entity_id.clone().map(Value::from)),
            ("argument", self.argument.clone().map(Value::from)),
            ("diagnostic", self.diagnostic.clone()),
            ("data", self.data.clone()),
        ] {
            if let Some(value) = value {
                error[key] = value;
            }
        }
        error
    }
}

/// `("E003", "unresolved …")` for `"E003: unresolved …"`: a leading
/// diagnostic code, a letter and three digits.
fn split_code(message: &str) -> Option<(&str, &str)> {
    let (code, rest) = message.split_once(": ")?;
    is_diagnostic_code(code).then_some((code, rest))
}

/// Whether `code` is a diagnostic code (`E003`, `R004`, `R-RES-006`):
/// capitals, digits and dashes, not a slug such as `extension_not_found`.
pub fn is_diagnostic_code(code: &str) -> bool {
    code.starts_with(|c: char| c.is_ascii_uppercase())
        && code.contains(|c: char| c.is_ascii_digit())
        && code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
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
    /// `diagnostics` ride in `_meta.diagnostics` and `meta`'s entries beside
    /// them in `_meta`; `events` are pushed before the reply goes out.
    Done {
        payload: Payload,
        is_error: bool,
        diagnostics: Vec<Diagnostic>,
        meta: serde_json::Map<String, Value>,
        events: Vec<(String, Value)>,
    },
    /// The tool failed: an `isError` result carrying the `McpError` (ADR
    /// 0004 D4-a). The one way a tool reports a failure.
    Refused(Box<McpError>),
}

impl ToolOutcome {
    fn done(payload: Payload, is_error: bool) -> Self {
        ToolOutcome::Done {
            payload,
            is_error,
            diagnostics: Vec::new(),
            meta: serde_json::Map::new(),
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

    /// A failed run whose output is a JSON object: a command's error
    /// object (ADR 0011).
    pub fn failed(payload: Value) -> Self {
        Self::done(Payload::Json(payload), true)
    }

    /// Plain-text blocks, failed or not: a command's output.
    pub fn texts(blocks: Vec<String>, is_error: bool) -> Self {
        Self::done(Payload::Text(blocks), is_error)
    }

    /// A failure with `code` and `message`.
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        McpError::new(code, message).into()
    }

    /// Invalid input: an argument the tool cannot use.
    pub fn invalid_input(argument: &str, message: impl Into<String>) -> Self {
        McpError::new(ErrorCode::InvalidInput, message)
            .with_argument(argument)
            .into()
    }

    /// A tool that needs a project and has none to work on.
    pub fn no_project(message: impl Into<String>) -> Self {
        Self::error(ErrorCode::PreconditionFailed, message)
    }

    /// The same outcome with `extra` added to its `_meta.diagnostics`.
    pub fn with_diagnostics(mut self, extra: Vec<Diagnostic>) -> Self {
        match &mut self {
            ToolOutcome::Done { diagnostics, .. } => diagnostics.extend(extra),
            ToolOutcome::Refused(error) => error.reported.extend(extra),
        }
        self
    }

    /// The same outcome with `value` under `key` in its `_meta` (a
    /// `specforge/`-prefixed key: MCP reserves `mcp` and
    /// `modelcontextprotocol`). A failure has no `_meta` but its
    /// diagnostics, so it is left as it is.
    pub fn with_meta(mut self, key: impl Into<String>, value: Value) -> Self {
        if let ToolOutcome::Done { meta, .. } = &mut self {
            meta.insert(key.into(), value);
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

    /// The same outcome, a failure naming `tool` unless it names one.
    pub fn from_tool(mut self, tool: &str) -> Self {
        if let ToolOutcome::Refused(error) = &mut self
            && error.tool.is_none()
        {
            error.tool = Some(tool.to_string());
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

    /// Take the events to push, leaving none.
    pub fn take_events(&mut self) -> Vec<(String, Value)> {
        match self {
            ToolOutcome::Done { events, .. } => std::mem::take(events),
            ToolOutcome::Refused(_) => Vec::new(),
        }
    }
}

impl From<McpError> for ToolOutcome {
    fn from(error: McpError) -> Self {
        ToolOutcome::Refused(Box::new(error))
    }
}

/// What a handler returns: an outcome, or a refusal it raised with `?`
/// (`call.project()?`): a [`Handled`].
pub trait IntoOutcome {
    fn into_outcome(self) -> ToolOutcome;
}

impl IntoOutcome for ToolOutcome {
    fn into_outcome(self) -> ToolOutcome {
        self
    }
}

impl IntoOutcome for Handled {
    fn into_outcome(self) -> ToolOutcome {
        self.unwrap_or_else(ToolOutcome::Refused)
    }
}

/// A handler's result when it refuses with `?`: the `McpError` boxed, as
/// [`ToolOutcome::Refused`] holds it (`call.project()?` converts).
pub type Handled = Result<ToolOutcome, Box<McpError>>;

/// The `tools/call` reply for `outcome`: the only place that builds
/// `content`, `structuredContent`, `isError` and `_meta`. A failure is an
/// `isError` result whose text is its `McpError`. With `structured` (a
/// 2025-06-18 or later session), a JSON object payload is also sent as
/// `structuredContent`, beside the text block holding its JSON; except a
/// failure of a tool with an outputSchema (`typed`), which the schema does
/// not describe.
pub fn envelope(
    outcome: ToolOutcome,
    id: Option<Value>,
    structured: bool,
    typed: bool,
) -> JsonRpcResponse {
    let (payload, is_error, diagnostics, mut meta) = match outcome {
        ToolOutcome::Refused(error) => {
            let json = error.to_json();
            (
                Payload::Json(json),
                true,
                error.reported,
                serde_json::Map::new(),
            )
        }
        ToolOutcome::Done {
            payload,
            is_error,
            diagnostics,
            meta,
            ..
        } => (payload, is_error, diagnostics, meta),
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
        && !(is_error && typed)
    {
        result["structuredContent"] = object;
    }
    if !diagnostics.is_empty() {
        meta.insert(
            "diagnostics".to_string(),
            serde_json::to_value(specforge_common::diagnostics_json(&diagnostics))
                .unwrap_or_default(),
        );
    }
    if !meta.is_empty() {
        result["_meta"] = Value::Object(meta);
    }
    JsonRpcResponse::success(id, result)
}
