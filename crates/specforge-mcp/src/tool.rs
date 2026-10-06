//! What a tool call produced, and the one place that turns it into a
//! `tools/call` reply.
//!
//! Handlers return a [`ToolOutcome`] (a mutation's handler, a
//! [`Mutated`](crate::mutation::Mutated) holding one); [`envelope`] alone
//! builds `content`, `isError` and `_meta`. What a mutation wrote crosses
//! to the dispatcher typed (ADR 0022), never read back from the reply.

use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity, codes};

use crate::mutation::Mutated;
use crate::protocol::{JsonRpcError, JsonRpcResponse, error_codes};
use crate::target::{Call, TargetSpec};
use crate::types::McpToolDescriptor;
use specforge_ops::{OpError, OpErrorKind};

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

/// How a tool is run: its handler, by role.
#[derive(Clone, Copy)]
pub enum Handler {
    /// Any tool but a mutation: its reply is all there is (collect and
    /// render write output artifacts, not project sources; spec feature
    /// `mcp_project_management_tools`).
    Tool(fn(&mut Call<'_>, Value) -> ToolOutcome),
    /// A mutation (category `mutation`): its reply and what it wrote.
    Mutation(fn(&mut Call<'_>, Value) -> Mutated),
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
    /// the arguments it reads, beside the ones its target reads
    /// ([`Self::reads`]).
    pub fields: fn() -> &'static [&'static str],
    /// Which project it acts on, and whether that project is brought up
    /// to date first: resolved into the call's target before the handler.
    pub target: TargetSpec,
    /// The handler, reading its `Args` from the call's `arguments`: a
    /// [`Handler::Mutation`] exactly for the `mutation` category.
    pub handler: Handler,
}

impl ToolSpec {
    /// The input schema `tools/list` lists: [`Self::schema`] with the
    /// target's properties merged in and its required arguments added.
    pub fn input_schema(&self) -> Value {
        let mut schema = (self.schema)();
        if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            properties.extend(self.target.properties());
        }
        let required = self.target.required();
        if !required.is_empty() {
            let listed = schema
                .as_object_mut()
                .map(|schema| schema.entry("required").or_insert_with(|| json!([])));
            if let Some(Value::Array(listed)) = listed {
                listed.extend(required.iter().map(|name| Value::from(*name)));
            }
        }
        schema
    }

    /// Every argument the call reads: the handler's `Args` fields, then the
    /// target's ([`TargetSpec::fields`]).
    pub fn reads(&self) -> Vec<&'static str> {
        (self.fields)()
            .iter()
            .chain(self.target.fields())
            .copied()
            .collect()
    }

    /// The tool as `tools/list` describes it.
    pub fn descriptor(&self) -> McpToolDescriptor {
        McpToolDescriptor {
            name: self.name.into(),
            description: self.description.into(),
            input_schema: self.input_schema(),
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

    /// The JSON-RPC error code a refusal is sent with where there is no
    /// `isError` result (`prompts/get`): invalid params for input the
    /// client can fix, invalid request before `initialize`, internal error
    /// for a failure on the server's side.
    pub fn rpc_code(self) -> i64 {
        match self {
            ErrorCode::InvalidInput
            | ErrorCode::EntityNotFound
            | ErrorCode::ExtensionNotFound
            | ErrorCode::Conflict => error_codes::INVALID_PARAMS,
            ErrorCode::NotInitialized => error_codes::INVALID_REQUEST,
            ErrorCode::CompilationFailed
            | ErrorCode::FileNotFound
            | ErrorCode::PermissionDenied
            | ErrorCode::Timeout
            | ErrorCode::SchemaMismatch
            | ErrorCode::InternalError
            | ErrorCode::PreconditionFailed => error_codes::INTERNAL_ERROR,
        }
    }

    /// The code a failure reported with diagnostic `code` carries: the
    /// kind operations give it ([`OpErrorKind::of_diagnostic`]).
    pub fn for_diagnostic(code: &str) -> Self {
        OpErrorKind::of_diagnostic(code).into()
    }
}

/// The code an operation's failure kind is reported as: total, one arm per
/// kind (ADR 0024 D15).
impl From<OpErrorKind> for ErrorCode {
    fn from(kind: OpErrorKind) -> Self {
        match kind {
            OpErrorKind::InvalidInput => ErrorCode::InvalidInput,
            OpErrorKind::EntityNotFound => ErrorCode::EntityNotFound,
            OpErrorKind::FileNotFound => ErrorCode::FileNotFound,
            OpErrorKind::ExtensionNotFound => ErrorCode::ExtensionNotFound,
            OpErrorKind::Conflict => ErrorCode::Conflict,
            OpErrorKind::SchemaMismatch => ErrorCode::SchemaMismatch,
            OpErrorKind::PreconditionFailed => ErrorCode::PreconditionFailed,
            OpErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            OpErrorKind::Timeout => ErrorCode::Timeout,
            OpErrorKind::Internal => ErrorCode::InternalError,
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
    /// The prompt that refused: a `prompts/get` answered with an error.
    pub prompt: Option<String>,
    /// The URI of the resource whose read failed: a `resources/read`
    /// answered with an error (the URI read).
    pub uri: Option<String>,
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
            prompt: None,
            uri: None,
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
            Some((code, rest)) => {
                Self::from_diagnostic(&Diagnostic::untyped(code, Severity::Error, rest))
            }
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

    /// The same error with `value` under `key` in its `data` object
    /// (created when it has none; a `data` that is no object is left as
    /// it is).
    pub fn with_data_field(mut self, key: &str, value: Value) -> Self {
        self.set_data_field(key, value);
        self
    }

    fn set_data_field(&mut self, key: &str, value: Value) {
        if let Value::Object(data) = self.data.get_or_insert_with(|| json!({})) {
            data.insert(key.to_string(), value);
        }
    }

    /// This refusal as the JSON-RPC error of a request that has no `isError`
    /// result (`prompts/get`, `resources/read`): -32602 when the client can
    /// fix it (it names an argument the client sent, or
    /// [`ErrorCode::rpc_code`] marks its code as input), else -32603; its
    /// data is this McpError (ADR 0004 D4-d, ADR 0024 D5).
    pub fn into_rpc_error(self) -> JsonRpcError {
        let code = if self.argument.is_some() {
            error_codes::INVALID_PARAMS
        } else {
            self.code.rpc_code()
        };
        JsonRpcError::new(code, self.message.clone()).with_data(self.to_json())
    }

    /// The error as its `isError` result carries it.
    pub fn to_json(&self) -> Value {
        let mut error = json!({ "code": self.code.as_str(), "message": self.message });
        for (key, value) in [
            ("tool", self.tool.clone().map(Value::from)),
            ("prompt", self.prompt.clone().map(Value::from)),
            ("uri", self.uri.clone().map(Value::from)),
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

/// An operation's failure as an `McpError`: its kind picks the code. A
/// diagnostic code (`E027`) rides in `diagnostic`, with its suggestion; a
/// slug's suggestion and the operation's own data ride in `data`; the
/// entity the failure is about is `entity_id`.
impl From<OpError> for McpError {
    fn from(error: OpError) -> Self {
        let mut mcp_error = McpError::new(error.kind.into(), error.message.clone());
        let mut data = error.data.map_or_else(|| json!({}), |data| *data);
        if is_diagnostic_code(&error.code) {
            let mut diagnostic =
                Diagnostic::untyped(error.code.as_ref(), Severity::Error, error.message);
            if let Some(suggestion) = error.suggestion {
                diagnostic = diagnostic.with_suggestion(suggestion);
            }
            mcp_error = mcp_error.with_diagnostic(&diagnostic);
        } else if let Some(suggestion) = error.suggestion {
            data["suggestion"] = Value::from(suggestion);
        }
        if data.as_object().is_some_and(|d| !d.is_empty()) {
            mcp_error = mcp_error.with_data(data);
        }
        match error.entity {
            Some(entity) => mcp_error.with_entity(entity),
            None => mcp_error,
        }
    }
}

/// A question about `entity_id`, which no entity of the graph declares:
/// `entity_not_found` naming it, its E003 in `diagnostic` (the one refusal
/// tools and prompts share).
pub fn entity_not_found(entity_id: &str) -> McpError {
    McpError::from_coded_message(
        ErrorCode::EntityNotFound,
        &format!(
            "{}: unresolved entity '{entity_id}' — not found in graph",
            codes::E003
        ),
    )
    .with_entity(entity_id)
}

/// What a refusal of a file the project does not hold says before the file's
/// name ([`file_not_found`]).
pub(crate) const FILE_NOT_FOUND: &str = "File not found: ";

/// A question about `file`, which the project has no entity from and does
/// not hold under its spec root: `file_not_found` on argument `file`.
pub(crate) fn file_not_found(file: &str) -> McpError {
    McpError::new(ErrorCode::FileNotFound, format!("{FILE_NOT_FOUND}{file}")).with_argument("file")
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
    /// them in `_meta`.
    Done {
        payload: Payload,
        is_error: bool,
        diagnostics: Vec<Diagnostic>,
        meta: serde_json::Map<String, Value>,
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

    /// The same outcome with `value` under `key`: in its payload when that
    /// is a JSON object, in its `McpError`'s `data` when it refused. A
    /// text payload is left as it is.
    pub(crate) fn with_field(mut self, key: &str, value: Value) -> Self {
        self.set_field(key, value);
        self
    }

    /// [`Self::with_field`], in place.
    pub(crate) fn set_field(&mut self, key: &str, value: Value) {
        match self {
            ToolOutcome::Done {
                payload: Payload::Json(Value::Object(object)),
                ..
            } => {
                object.insert(key.to_string(), value);
            }
            ToolOutcome::Done { .. } => {}
            ToolOutcome::Refused(error) => error.set_data_field(key, value),
        }
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

impl IntoOutcome for McpError {
    fn into_outcome(self) -> ToolOutcome {
        self.into()
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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [ErrorCode; 12] = [
        ErrorCode::InvalidInput,
        ErrorCode::CompilationFailed,
        ErrorCode::EntityNotFound,
        ErrorCode::FileNotFound,
        ErrorCode::ExtensionNotFound,
        ErrorCode::PermissionDenied,
        ErrorCode::Timeout,
        ErrorCode::NotInitialized,
        ErrorCode::SchemaMismatch,
        ErrorCode::InternalError,
        ErrorCode::Conflict,
        ErrorCode::PreconditionFailed,
    ];

    #[test]
    fn every_operation_kind_has_an_error_code() {
        for (kind, code) in [
            (OpErrorKind::InvalidInput, ErrorCode::InvalidInput),
            (OpErrorKind::EntityNotFound, ErrorCode::EntityNotFound),
            (OpErrorKind::FileNotFound, ErrorCode::FileNotFound),
            (OpErrorKind::ExtensionNotFound, ErrorCode::ExtensionNotFound),
            (OpErrorKind::Conflict, ErrorCode::Conflict),
            (OpErrorKind::SchemaMismatch, ErrorCode::SchemaMismatch),
            (
                OpErrorKind::PreconditionFailed,
                ErrorCode::PreconditionFailed,
            ),
            (OpErrorKind::PermissionDenied, ErrorCode::PermissionDenied),
            (OpErrorKind::Timeout, ErrorCode::Timeout),
            (OpErrorKind::Internal, ErrorCode::InternalError),
        ] {
            assert_eq!(ErrorCode::from(kind), code, "{kind:?}");
        }
    }

    #[test]
    fn an_operation_failure_is_an_mcp_error_by_its_kind() {
        let error: McpError = OpError::new(OpErrorKind::Conflict, "entity_exists", "taken")
            .with_entity("alpha")
            .with_suggestion("pick another")
            .into();
        let json = error.to_json();
        assert_eq!(json["code"], "conflict");
        assert_eq!(json["entity_id"], "alpha");
        assert_eq!(json["data"]["suggestion"], "pick another");
        assert!(json.get("diagnostic").is_none(), "{json}");

        let error: McpError = OpError::diagnostic(codes::E062, "the budget is too small")
            .with_suggestion("raise it")
            .into();
        let json = error.to_json();
        assert_eq!(json["code"], "invalid_input");
        assert_eq!(json["diagnostic"]["code"], "E062");
        assert_eq!(json["diagnostic"]["suggestion"], "raise it");
        assert_eq!(json["message"], "the budget is too small");
    }

    #[test]
    fn every_error_code_has_a_json_rpc_code() {
        for code in ALL {
            let rpc = code.rpc_code();
            let expected = match code.as_str() {
                "invalid_input" | "entity_not_found" | "extension_not_found" | "conflict" => {
                    error_codes::INVALID_PARAMS
                }
                "not_initialized" => error_codes::INVALID_REQUEST,
                _ => error_codes::INTERNAL_ERROR,
            };
            assert_eq!(rpc, expected, "{}", code.as_str());
        }
    }

    #[test]
    fn a_prompts_refusal_names_the_prompt() {
        let mut error = McpError::new(ErrorCode::InvalidInput, "no");
        assert!(error.to_json().get("prompt").is_none());
        error.prompt = Some("specforge://prompts/context".into());
        assert_eq!(error.to_json()["prompt"], "specforge://prompts/context");
        assert!(error.to_json().get("tool").is_none());
    }

    #[test]
    fn a_resources_refusal_names_the_uri() {
        let mut error = McpError::new(ErrorCode::InvalidInput, "no");
        assert!(error.to_json().get("uri").is_none());
        error.uri = Some("specforge://graph?depth=two".into());
        assert_eq!(error.to_json()["uri"], "specforge://graph?depth=two");
        assert!(error.to_json().get("resource").is_none());
        assert!(error.to_json().get("prompt").is_none());
    }

    #[test]
    fn a_refusal_without_is_error_is_invalid_params_when_the_client_can_fix_it() {
        let rpc = |error: McpError| error.into_rpc_error();
        // An argument it named, whatever its code.
        let named = McpError::new(ErrorCode::FileNotFound, "path not found").with_argument("path");
        let named = rpc(named);
        assert_eq!(named.code, error_codes::INVALID_PARAMS);
        assert_eq!(named.data.as_ref().unwrap()["argument"], "path");
        // A code that is input.
        assert_eq!(
            rpc(McpError::new(ErrorCode::EntityNotFound, "no")).code,
            error_codes::INVALID_PARAMS
        );
        // A failure on the server's side, and a missing project (no params
        // fix it).
        assert_eq!(
            rpc(McpError::new(ErrorCode::InternalError, "no")).code,
            error_codes::INTERNAL_ERROR
        );
        assert_eq!(
            rpc(McpError::new(ErrorCode::PreconditionFailed, "no")).code,
            error_codes::INTERNAL_ERROR
        );
    }

    #[test]
    fn an_unknown_entity_carries_its_e003() {
        let json = entity_not_found("ghost").to_json();
        assert_eq!(json["code"], "entity_not_found");
        assert_eq!(json["entity_id"], "ghost");
        assert_eq!(json["diagnostic"]["code"], "E003");
        assert_eq!(
            json["message"],
            "unresolved entity 'ghost' — not found in graph"
        );
    }
}
