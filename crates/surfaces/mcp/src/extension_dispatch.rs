//! Extension dispatch: a `tools/call` or `resources/read` the extension
//! surface table answers, run by one of three adapters, each one call over
//! the `WasmRuntime` seam the call's project was compiled in (ADR 0017 D8,
//! ADR 0013): an explicit tool's `mcp__` export ([`mcp_tool_adapter`]), a
//! command's `cmd__` export ([`command_adapter`]) and a resource's `mcp__`
//! export ([`resource_adapter`]). The dispatch event is recorded through the
//! call's event sink when the export returned, whatever it answered.

use serde_json::{Value, json};
use specforge_ops::extension::command::{CommandFormat, ExtensionCommand, RunError};
use specforge_protocol_types::command_args::ArgError;
use specforge_protocol_types::{CommandError, CommandOutput};
use specforge_wasm::runtime::WasmRuntime;

use crate::events::McpEvent;
use crate::reply::Source;
use crate::resource::{ReadOutcome, ResourceText};
use crate::surface_table::{ResourceEntry, ToolEntry, ToolKind};
use crate::target::{Call, ProjectRef};
use crate::tool::{ErrorCode, McpError, ToolOutcome};

/// The extension tool `entry`, called with `arguments` over the call's
/// project: an explicit tool's arguments its declared schema refuses are
/// refused first, the project resolved after; a command's are normalized by
/// its run.
pub(crate) fn run_tool(call: &mut Call<'_>, entry: &ToolEntry, arguments: &Value) -> ToolOutcome {
    let (outcome, dispatched) = match &entry.kind {
        ToolKind::McpTool {
            export,
            input_schema,
            output_schema,
        } => {
            if let Err(refused) = check_input(input_schema, arguments) {
                return refused;
            }
            match call.project() {
                Ok(project) => mcp_tool_adapter(
                    project.runtime.as_ref(),
                    entry,
                    export,
                    output_schema.as_ref(),
                    arguments,
                ),
                Err(refused) => (refused.into(), None),
            }
        }
        ToolKind::Command(command) if command.writes() => {
            return writing::run(call, entry, command, arguments);
        }
        ToolKind::Command(command) => match call.project() {
            Ok(project) => command_adapter(&project, command, arguments),
            Err(refused) => (refused.into(), None),
        },
    };
    if let Some(event) = dispatched {
        call.events().record(event);
    }
    outcome
}

/// The extension resource `entry`, read at `uri` over the call's project. A
/// failed read is the read's error (E028 in `diagnostic`), and no dispatch
/// is recorded.
pub(crate) fn read_resource(call: &mut Call<'_>, entry: &ResourceEntry, uri: &str) -> ReadOutcome {
    // The project the resource reads, in the runtime it was compiled in.
    let (outcome, dispatched) = match call.project() {
        Ok(project) => resource_adapter(project.runtime.as_ref(), entry, uri),
        Err(refused) => (Err(Box::new(refused)), None),
    };
    if let Some(event) = dispatched {
        call.events().record(event);
    }
    outcome
}

/// An explicit tool's arguments checked against the input schema it
/// declares, before its module runs: `invalid_input` naming each
/// violation (its schema is opaque JSON to the host, ADR 0004 D4-a).
fn check_input(schema: &Value, arguments: &Value) -> Result<(), ToolOutcome> {
    let violations = specforge_common::shape::violations(schema, arguments);
    if violations.is_empty() {
        return Ok(());
    }
    Err(McpError::new(
        ErrorCode::InvalidInput,
        format!(
            "the arguments do not match the tool's input schema: {}",
            violations.join("; ")
        ),
    )
    .with_data(json!({ "violations": violations }))
    .into())
}

/// An explicit tool: its `mcp__` export called with the arguments, its
/// output checked against the output schema it declares. The tool's
/// outcome, and its dispatch event when its export returned.
fn mcp_tool_adapter(
    runtime: &dyn WasmRuntime,
    entry: &ToolEntry,
    export: &str,
    output_schema: Option<&Value>,
    arguments: &Value,
) -> (ToolOutcome, Option<McpEvent>) {
    let started = std::time::Instant::now();
    let result = match specforge_wasm::ExtensionCalls::new(runtime).call_mcp_tool(
        &entry.extension,
        export,
        arguments,
    ) {
        Ok(value) => match output_schema {
            // An output the tool's own schema refuses is never served as
            // its structured result.
            Some(schema) => crate::reply::conforming(
                &entry.name,
                Source::Extension(&entry.extension),
                schema,
                ToolOutcome::ok(value),
            ),
            None => ToolOutcome::ok(value),
        },
        // A failed call is the tool's error, and no dispatch is recorded.
        Err(error) => return (failed_call(&error.diagnostic()).into(), None),
    };
    // A tool whose export returned is a dispatched tool; it succeeded when
    // its output is the tool's result.
    let event = McpEvent::SurfaceMcpToolDispatched {
        extension_name: entry.extension.clone(),
        tool_name: entry.name.clone(),
        duration_ms: elapsed_ms(started),
        success: result.succeeded(),
    };
    (result, Some(event))
}

/// A command's run as a tool result, and its dispatch event when its export
/// returned (whatever its exit code). The run is `specforge <ext> <command>`
/// over the call's project, always asked for json: the tool has no format
/// argument (ADR 0011 A). Args the rule refuses are `invalid_input` on the arg
/// ([`refused_args`], ADR 0252); an export that did not answer is a
/// structured MCP error carrying its E028 (ADR 0013 D4); no dispatch is
/// recorded for either.
fn command_adapter(
    project: &ProjectRef<'_>,
    command: &ExtensionCommand,
    arguments: &Value,
) -> (ToolOutcome, Option<McpEvent>) {
    let given = arguments.as_object().cloned().unwrap_or_default();
    let started = std::time::Instant::now();
    match specforge_ops::extension::command::run(
        &project.view(),
        command,
        &given,
        CommandFormat::Json,
    ) {
        Ok(output) => {
            let event = McpEvent::SurfaceCommandDispatched {
                extension_name: command.extension().to_string(),
                command_id: command.id().to_string(),
                exit_code: output.exit_code,
                duration_ms: elapsed_ms(started),
            };
            (output_result(command, output), Some(event))
        }
        Err(RunError::Args(refused)) => (refused_args(&refused).into(), None),
        Err(RunError::Call(error)) => (failed_call(&error.diagnostic()).into(), None),
        Err(RunError::NoProject(error)) => (McpError::from(error).into(), None),
    }
}

/// The one arg rule's refusal as the McpError a core tool's own reading makes:
/// `invalid_input` on the arg it names (the first missing one, for several),
/// the rule's message (the CLI's), the close arg or value as
/// `data.suggestion` (ADR 0252 D2); a missing one in every tool's sentence (ADR 0275 D2).
fn refused_args(error: &ArgError) -> McpError {
    if let ArgError::Missing { names } = error
        && let Some(first) = names.first()
    {
        return crate::args::missing(first);
    }
    let argument = match error {
        ArgError::Missing { names } => names.first(),
        ArgError::NotOneOf { name, .. }
        | ArgError::NotInteger { name, .. }
        | ArgError::NotString { name, .. }
        | ArgError::NotBool { name, .. }
        | ArgError::Unknown { name, .. } => Some(name),
    };
    let mut refused = McpError::new(ErrorCode::InvalidInput, error.message());
    if let Some(argument) = argument {
        refused = refused.with_argument(argument);
    }
    match error.suggestion() {
        Some(close) => {
            refused.with_data_field("suggestion", json!(format!("did you mean '{close}'?")))
        }
        None => refused,
    }
}

/// An extension resource read through its `mcp__` export. A failed read (a
/// trap, an answer that is not its content and MIME type, an export the
/// guest does not route) is a server-side fault of the extension: an
/// internal error (-32603) whose `data` is the `McpError` carrying the E028
/// diagnostic (D6, `mcp_structured_error_responses`).
fn resource_adapter(
    runtime: &dyn WasmRuntime,
    entry: &ResourceEntry,
    uri: &str,
) -> (ReadOutcome, Option<McpEvent>) {
    let started = std::time::Instant::now();
    match specforge_wasm::ExtensionCalls::new(runtime).read_mcp_resource(
        &entry.extension,
        &entry.export,
        uri,
    ) {
        // A read whose export answered its content is a dispatched
        // resource; a failed call is the read's error, and no dispatch is
        // recorded.
        Ok(read) => {
            let event = McpEvent::SurfaceMcpResourceDispatched {
                extension_name: entry.extension.clone(),
                uri_template: entry.uri_template.clone(),
                mime_type: read.mime_type.clone(),
                duration_ms: elapsed_ms(started),
            };
            let text = ResourceText::new(read.mime_type, read.content);
            (Ok(text), Some(event))
        }
        Err(error) => (Err(Box::new(failed_call(&error.diagnostic()))), None),
    }
}

/// A command's own error object as the McpError its code names (ADR 0252 D3):
/// `INVALID_INPUT` is `invalid_input`, `ENTITY_NOT_FOUND` is `entity_not_found`,
/// any other code is `internal_error` keeping it in `data.code`; its entity is
/// kept, its suggestion rides in `data.suggestion` as a core tool's does.
fn own_error(error: CommandError) -> McpError {
    let code = match error.code.as_str() {
        "INVALID_INPUT" => ErrorCode::InvalidInput,
        "ENTITY_NOT_FOUND" => ErrorCode::EntityNotFound,
        _ => ErrorCode::InternalError,
    };
    let mut refused = McpError::new(code, error.message);
    if code == ErrorCode::InternalError {
        refused = refused.with_data_field("code", json!(error.code));
    }
    if let Some(entity) = error.entity_id {
        refused = refused.with_entity(entity);
    }
    match error.suggestion {
        Some(close) => {
            refused.with_data_field("suggestion", json!(format!("did you mean '{close}'?")))
        }
        None => refused,
    }
}

/// A command's failed run as an McpError (ADR 0252 D3, D4): its one error
/// object on stderr, nothing on stdout, is its [`own_error`]; any other output
/// is `internal_error` "command <export>() of '<extension>' exited <n>[: <stderr>]"
/// carrying `data.{exit_code, stdout, stderr}`.
fn failed_run(command: &ExtensionCommand, output: CommandOutput) -> McpError {
    if output.stdout.is_empty()
        && let Ok(error) = serde_json::from_str::<CommandError>(&output.stderr)
    {
        return own_error(error);
    }
    let shown = output.stderr.trim();
    let mut message = format!(
        "command {}() of '{}' exited {}",
        command.export(),
        command.extension(),
        output.exit_code
    );
    if !shown.is_empty() {
        message = format!("{message}: {shown}");
    }
    McpError::new(ErrorCode::InternalError, message).with_data(json!({
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
    }))
}

/// An auto-promoted command's output as a tool result. The command was
/// asked for json (ADR 0011): a JSON object on stdout, and nothing on
/// stderr, is the result's structured payload; a nonzero exit code is a
/// [`failed_run`]. Otherwise its stdout, then its stderr when it wrote any.
fn output_result(command: &ExtensionCommand, output: CommandOutput) -> ToolOutcome {
    if output.exit_code != 0 {
        return failed_run(command, output).into();
    }
    if output.stderr.is_empty()
        && let Ok(payload @ Value::Object(_)) = serde_json::from_str::<Value>(&output.stdout)
    {
        return ToolOutcome::ok(payload);
    }
    let mut blocks = vec![output.stdout];
    if !output.stderr.is_empty() {
        blocks.push(output.stderr);
    }
    ToolOutcome::texts(blocks)
}

/// A failed extension call as an MCP error: the diagnostic the runtime
/// reported, in `diagnostic`.
fn failed_call(diagnostic: &specforge_common::Diagnostic) -> McpError {
    McpError::new(ErrorCode::InternalError, diagnostic.message.clone()).with_diagnostic(diagnostic)
}

/// How long since `started`, in whole milliseconds.
fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(crate) mod writing;

#[cfg(test)]
mod tests;
