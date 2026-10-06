//! Every operation the host performs on a loaded extension, typed (ADR
//! 0013).
//!
//! The host calls ten kinds of export: the handshake and describe that read
//! an extension's declaration, a command, an MCP tool, an MCP resource, a
//! compiler pass, a collector, a custom validator, a scanner and the
//! migration hook. [`ExtensionCalls`] has one function per operation: its
//! input and its answer are the protocol's types
//! (`specforge_protocol_types`, the ones the SDK shares), and every failure
//! is one [`CallError`], E028. Callers never see bytes, export prefixes or
//! traps: encoding, strict decoding, the export a pass is called by, the
//! canonical order of a pass's diagnostics and the mapping of every failure
//! live here, over the byte-level [`WasmRuntime`] port.
//!
//! What a failure means stays the caller's: a check pass's is a compile
//! diagnostic, an analyze pass's a finding of that pass, a command's its
//! error, a scanner's a reported failure, a collector's the collect error.

use std::fmt;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use specforge_common::{Diagnostic, DiagnosticData, Severity, SourceSpan, Sym};
use specforge_protocol_types::{
    CollectInput, CollectOutput, CommandInput, CommandOutput, DescribeRequest, DescribeResponse,
    HandshakeRequest, HandshakeResponse, McpResourceContent, McpResourceRequest, MigrationInput,
    PROTOCOL_VERSION, PassAnswer, PassDiagnostic, PassInput, PassOutput, PassSeverity, PassSpan,
    RawGraph, SUPPORTED_CATEGORIES, ScanRequest, ScanResponse, ValidatorContext, ValidatorVerdict,
};

use crate::runtime::{WasmCallResult, WasmRuntime};

/// The operation a call performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Handshake,
    Describe,
    Command,
    McpTool,
    McpResource,
    Pass,
    Collect,
    Validate,
    Scan,
    Migrate,
}

impl Operation {
    /// Every operation.
    pub const ALL: [Operation; 10] = [
        Operation::Handshake,
        Operation::Describe,
        Operation::Command,
        Operation::McpTool,
        Operation::McpResource,
        Operation::Pass,
        Operation::Collect,
        Operation::Validate,
        Operation::Scan,
        Operation::Migrate,
    ];

    /// How a message names the operation.
    pub fn label(self) -> &'static str {
        match self {
            Operation::Handshake => "handshake",
            Operation::Describe => "describe",
            Operation::Command => "command",
            Operation::McpTool => "MCP tool",
            Operation::McpResource => "MCP resource",
            Operation::Pass => "compiler pass",
            Operation::Collect => "collector",
            Operation::Validate => "custom validator",
            Operation::Scan => "scanner",
            Operation::Migrate => "migration hook",
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Why a call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallFailure {
    /// The runtime has no extension of that name (it did not load it).
    NotLoaded,
    /// The export did not answer: it trapped, the guest returned an error
    /// (`guest_error`, which an export the guest does not route is too), or
    /// its time ran out (`deadline_exceeded`).
    Trapped { kind: String, message: String },
    /// The export answered, but not the protocol type it owes.
    Malformed {
        expected: &'static str,
        reason: String,
    },
    /// The host's input did not encode (a host bug; never sent).
    Unencodable { reason: String },
}

/// A failed extension call: E028 naming the operation, the export and the
/// extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallError {
    pub operation: Operation,
    pub extension: String,
    pub export: String,
    pub failure: CallFailure,
}

impl CallError {
    pub fn new(
        operation: Operation,
        extension: &str,
        export: &str,
        failure: CallFailure,
    ) -> CallError {
        CallError {
            operation,
            extension: extension.to_string(),
            export: export.to_string(),
            failure,
        }
    }

    /// The E028 diagnostic: `<operation> <export>() of '<extension>'`, then
    /// what went wrong, with the suggestion to report it to the extension's
    /// author.
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error("E028", self.to_string()).with_suggestion(format!(
            "report the failure to the author of '{}', or check it is installed and up to date",
            self.extension
        ))
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}() of '{}' ",
            self.operation, self.export, self.extension
        )?;
        match &self.failure {
            CallFailure::NotLoaded => f.write_str("is not loaded"),
            CallFailure::Trapped { kind, message } => write!(f, "trapped: {kind}: {message}"),
            CallFailure::Malformed { expected, reason } => {
                write!(f, "answered output that is not a {expected}: {reason}")
            }
            CallFailure::Unencodable { reason } => {
                write!(f, "was not called: its input does not encode: {reason}")
            }
        }
    }
}

impl std::error::Error for CallError {}

/// An input encoded once, to send to many exports (one pass input, every
/// pass).
#[derive(Debug, Clone)]
pub struct Encoded<T> {
    bytes: Vec<u8>,
    _type: PhantomData<fn(&T)>,
}

impl<T> Encoded<T> {
    /// The encoded bytes, as the guest receives them.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// The operations on loaded extensions, over a runtime; see the module
/// docs.
#[derive(Clone, Copy)]
pub struct ExtensionCalls<'r> {
    runtime: &'r dyn WasmRuntime,
}

impl<'r> ExtensionCalls<'r> {
    pub fn new(runtime: &'r dyn WasmRuntime) -> Self {
        ExtensionCalls { runtime }
    }

    /// `value` encoded once, for [`Self::run_pass`]. Err: it does not
    /// encode (a host bug), to report as each call's
    /// [`CallFailure::Unencodable`].
    pub fn encode<T: Serialize>(value: &T) -> Result<Encoded<T>, CallFailure> {
        serde_json::to_vec(value)
            .map(|bytes| Encoded {
                bytes,
                _type: PhantomData,
            })
            .map_err(|e| CallFailure::Unencodable {
                reason: e.to_string(),
            })
    }

    /// `__handshake`: the extension's identity and what it declares. The
    /// wall-clock budget it declares (`sandbox_policy.max_execution_ms`)
    /// gates its later calls.
    pub fn handshake(&self, extension: &str) -> Result<HandshakeResponse, CallError> {
        let request = HandshakeRequest {
            host_version: PROTOCOL_VERSION.to_string(),
            supported_categories: SUPPORTED_CATEGORIES.iter().map(|s| s.to_string()).collect(),
        };
        let response: HandshakeResponse = self.call_typed(
            Operation::Handshake,
            extension,
            "__handshake",
            &request,
            "HandshakeResponse",
        )?;
        if let Some(ms) = response
            .sandbox_policy
            .as_ref()
            .and_then(|policy| policy.max_execution_ms)
        {
            self.runtime
                .set_execution_deadline_ms(extension, u64::from(ms));
        }
        Ok(response)
    }

    /// `__describe` of `category`.
    pub fn describe(&self, extension: &str, category: &str) -> Result<DescribeResponse, CallError> {
        let request = DescribeRequest {
            category: category.to_string(),
        };
        self.call_typed(
            Operation::Describe,
            extension,
            "__describe",
            &request,
            "DescribeResponse",
        )
    }

    /// A command's `cmd__` export on `input`, whose graph the host already
    /// rendered.
    pub fn run_command(
        &self,
        extension: &str,
        export: &str,
        input: &CommandInput<RawGraph>,
    ) -> Result<CommandOutput, CallError> {
        self.call_typed(
            Operation::Command,
            extension,
            export,
            input,
            "CommandOutput",
        )
    }

    /// An MCP tool's export on `arguments`: any JSON value is its answer
    /// (the tool's output schema is the caller's to check).
    pub fn call_mcp_tool(
        &self,
        extension: &str,
        export: &str,
        arguments: &Value,
    ) -> Result<Value, CallError> {
        self.call_typed(
            Operation::McpTool,
            extension,
            export,
            arguments,
            "JSON value",
        )
    }

    /// An MCP resource's export, reading `uri`.
    pub fn read_mcp_resource(
        &self,
        extension: &str,
        export: &str,
        uri: &str,
    ) -> Result<McpResourceContent, CallError> {
        let request = McpResourceRequest {
            uri: uri.to_string(),
        };
        self.call_typed(
            Operation::McpResource,
            extension,
            export,
            &request,
            "McpResourceContent",
        )
    }

    /// The pass `pass`'s `__pass_<pass>` export on `input`: its diagnostics,
    /// with its summary (empty for a bare answer).
    pub fn run_pass(
        &self,
        extension: &str,
        pass: &str,
        input: &Encoded<PassInput>,
    ) -> Result<PassOutput, CallError> {
        let export = format!("__pass_{pass}");
        let answer: PassAnswer = self.call(
            Operation::Pass,
            extension,
            &export,
            input.as_bytes(),
            "PassAnswer",
        )?;
        Ok(answer.into_output())
    }

    /// A collector's `collect__` export on the report files.
    pub fn collect(
        &self,
        extension: &str,
        export: &str,
        input: &CollectInput,
    ) -> Result<CollectOutput, CallError> {
        self.call_typed(
            Operation::Collect,
            extension,
            export,
            input,
            "CollectOutput",
        )
    }

    /// A custom rule's `function` on one entity's context.
    pub fn validate(
        &self,
        extension: &str,
        function: &str,
        context: &ValidatorContext,
    ) -> Result<ValidatorVerdict, CallError> {
        self.call_typed(
            Operation::Validate,
            extension,
            function,
            context,
            "ValidatorVerdict",
        )
    }

    /// An analyzer's scan export on one source file.
    pub fn scan(
        &self,
        extension: &str,
        export: &str,
        request: &ScanRequest,
    ) -> Result<ScanResponse, CallError> {
        self.call_typed(Operation::Scan, extension, export, request, "ScanResponse")
    }

    /// The migration hook `hook`. Its answer is not read: the protocol
    /// defines none.
    pub fn migrate(
        &self,
        extension: &str,
        hook: &str,
        input: &MigrationInput,
    ) -> Result<(), CallError> {
        let bytes = encode(Operation::Migrate, extension, hook, input)?;
        self.call_raw(Operation::Migrate, extension, hook, &bytes)
            .map(|_| ())
    }

    fn call_typed<I: Serialize, O: DeserializeOwned>(
        &self,
        operation: Operation,
        extension: &str,
        export: &str,
        input: &I,
        expected: &'static str,
    ) -> Result<O, CallError> {
        let bytes = encode(operation, extension, export, input)?;
        self.call(operation, extension, export, &bytes, expected)
    }

    /// The one place bytes cross: call the export, map a trap, decode the
    /// answer as `O`.
    fn call<O: DeserializeOwned>(
        &self,
        operation: Operation,
        extension: &str,
        export: &str,
        bytes: &[u8],
        expected: &'static str,
    ) -> Result<O, CallError> {
        let answer = self.call_raw(operation, extension, export, bytes)?;
        serde_json::from_slice(&answer).map_err(|e| {
            CallError::new(
                operation,
                extension,
                export,
                CallFailure::Malformed {
                    expected,
                    reason: e.to_string(),
                },
            )
        })
    }

    fn call_raw(
        &self,
        operation: Operation,
        extension: &str,
        export: &str,
        bytes: &[u8],
    ) -> Result<Vec<u8>, CallError> {
        match self.runtime.call_export(extension, export, bytes) {
            WasmCallResult::Ok(answer) => Ok(answer),
            WasmCallResult::Trap(trap) => {
                let failure = if trap.kind == "extension_not_found" {
                    CallFailure::NotLoaded
                } else {
                    CallFailure::Trapped {
                        kind: trap.kind,
                        message: trap.message,
                    }
                };
                Err(CallError::new(operation, extension, export, failure))
            }
        }
    }
}

fn encode<T: Serialize>(
    operation: Operation,
    extension: &str,
    export: &str,
    input: &T,
) -> Result<Vec<u8>, CallError> {
    ExtensionCalls::encode(input)
        .map(|encoded| encoded.bytes)
        .map_err(|failure| CallError::new(operation, extension, export, failure))
}

/// The host diagnostics of a pass's answer, in canonical order (code, then
/// file and line, then message), whatever order the guest built them in. A
/// diagnostic without a span that names an entity gets that entity's
/// (`span_of`); one that names an entity carries it as
/// `DiagnosticData::Subject`.
pub fn pass_diagnostics(
    output: PassOutput,
    span_of: impl Fn(&str) -> Option<SourceSpan>,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = output
        .diagnostics
        .into_iter()
        .map(|diagnostic| {
            let PassDiagnostic {
                code,
                severity,
                message,
                span,
                suggestion,
                entity,
            } = diagnostic;
            let span = span
                .map(source_span)
                .or_else(|| entity.as_deref().and_then(&span_of));
            Diagnostic {
                code,
                severity: match severity {
                    PassSeverity::Error => Severity::Error,
                    PassSeverity::Warning => Severity::Warning,
                    PassSeverity::Info => Severity::Info,
                },
                message,
                span,
                suggestion,
                // The entity the guest names is the diagnostic's subject,
                // even one the graph lacks: navigation reads data, never the
                // message (ADR 0016). A guest's own `data` is not carried.
                data: entity.map(|entity| Box::new(DiagnosticData::Subject { entity })),
            }
        })
        .collect();
    diagnostics.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| {
                a.span
                    .as_ref()
                    .map(|s| (s.file.as_str(), s.start_line))
                    .cmp(&b.span.as_ref().map(|s| (s.file.as_str(), s.start_line)))
            })
            .then_with(|| a.message.cmp(&b.message))
    });
    diagnostics
}

fn source_span(span: PassSpan) -> SourceSpan {
    SourceSpan {
        file: Sym::new(&span.file),
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
    }
}
