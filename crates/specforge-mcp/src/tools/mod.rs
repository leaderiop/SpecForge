mod analyze;
pub(crate) mod coverage;
mod explain;
mod export;
mod find_definition;
mod find_implementation;
mod find_references;
pub(crate) mod find_spec_for_source;
mod infer_gaps;
mod infer_progress;
mod infer_session;
mod inspect;
mod list;
mod model;
mod outline;
mod outline_extensions;
mod query;
mod schema;
mod search;
mod stats;
mod suggest_fixes;
mod table;
pub(crate) mod trace;
mod validate;

use serde_json::{Value, json};

use crate::mutation::{self, Mutated};
use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::surface_table::{ToolEntry, ToolKind};
use crate::target::{self, Call, TargetSpec};
use crate::tool::{ErrorCode, Handler, McpError, ToolOutcome, ToolSpec, envelope};
pub use table::CORE_TOOLS;

/// The navigator over what the call reads (`specforge_ops::navigate`):
/// its project's view, else the empty session's graph without a root, each
/// file's text read from disk under the spec root (with no project, no
/// file is read). The navigation tools render its answers as JSON and
/// nothing else (ADR 0016).
pub(crate) fn navigator<'c>(
    call: &'c Call<'_>,
) -> specforge_ops::navigate::Navigator<'c, impl Fn(&str) -> Option<String> + 'c> {
    let spec_root = call.spec_root().map(std::path::Path::to_path_buf);
    specforge_ops::navigate::Navigator::new(call.view(), move |file| {
        std::fs::read_to_string(spec_root.as_ref()?.join(file)).ok()
    })
}

/// A span as the MCP tools render it: the `SourceSpan` the spec types name
/// (1-based lines, 1-based byte columns, end exclusive).
pub(crate) fn span_json(span: &specforge_common::SourceSpan) -> Value {
    json!({
        "file": span.file,
        "start_line": span.start_line,
        "start_col": span.start_col,
        "end_line": span.end_line,
        "end_col": span.end_col,
    })
}

/// An I020 report for each kind in a `kinds` filter that no registered
/// extension defines and no entity has, in the order given, with a
/// `did you mean` suggestion when a known kind is close. The filter still
/// drops them: they match no entity.
pub(crate) fn unknown_kind_diagnostics(
    state: &McpState,
    kinds: &[&str],
) -> Vec<specforge_common::Diagnostic> {
    let mut known: Vec<&str> = state
        .registries()
        .kinds
        .keywords()
        .map(String::as_str)
        .chain(
            state
                .graph()
                .nodes()
                .into_iter()
                .map(|n| n.kind.raw.as_str()),
        )
        .collect();
    known.sort_unstable();
    known.dedup();

    let mut reported: Vec<&str> = Vec::new();
    let mut diagnostics = Vec::new();
    for &kind in kinds {
        if known.binary_search(&kind).is_ok() || reported.contains(&kind) {
            continue;
        }
        reported.push(kind);
        let mut diag =
            specforge_common::Diagnostic::info("I020", format!("unknown entity kind '{kind}'"));
        if let Some(close) = specforge_common::find_close_match(kind, known.iter().copied()) {
            diag = diag.with_suggestion(format!("did you mean '{close}'?"));
        }
        diagnostics.push(diag);
    }
    diagnostics
}

/// An emitter failure about `entity_id` as a failed tool result. A
/// missing entity is [`entity_not_found`](crate::tool::entity_not_found),
/// its `E003` in `diagnostic`, never only in the message text.
fn emitter_error(error: specforge_emitter::EmitterError, entity_id: &str) -> ToolOutcome {
    use specforge_emitter::EmitterError;
    let mcp_error = match &error {
        EmitterError::EntityNotFound(_) => crate::tool::entity_not_found(entity_id),
        EmitterError::SerializationError(message) => {
            McpError::new(ErrorCode::InternalError, message.as_str())
        }
        EmitterError::InvalidScope(message) | EmitterError::Other(message) => {
            McpError::from_coded_message(ErrorCode::InvalidInput, message)
        }
    };
    mcp_error.into()
}

/// A project file the tool reads (`specforge-infer.json`, the anchors
/// manifest) that it cannot use: unreadable, or not what it should hold.
pub(crate) fn manifest_error(message: String) -> ToolOutcome {
    manifest_mcp_error(message).into()
}

/// [`manifest_error`]'s `McpError`: what a prompt that reads the file
/// refuses with.
pub(crate) fn manifest_mcp_error(message: String) -> McpError {
    let code = if message.starts_with("failed to read") {
        ErrorCode::InternalError
    } else {
        ErrorCode::SchemaMismatch
    };
    McpError::new(code, message)
}

/// A failed extension call as a failed tool result: the diagnostic the
/// runtime reported, in `diagnostic`.
fn extension_error(diag: &specforge_common::Diagnostic) -> ToolOutcome {
    McpError::from_diagnostic(diag).into()
}

/// An auto-promoted command's run as a tool result. The command was asked
/// for json (ADR 0011): a JSON object on stdout, and nothing on stderr, is
/// the result's structured payload; a failure that wrote one JSON object
/// on stderr, and nothing on stdout, is an `isError` result carrying it.
/// Otherwise its stdout, then its stderr when it wrote any; a nonzero exit
/// code fails the call.
fn command_tool_result(
    outcome: Result<specforge_protocol_types::CommandOutput, specforge_wasm::CallError>,
) -> ToolOutcome {
    let object = |text: &str| match serde_json::from_str::<Value>(text) {
        Ok(object @ Value::Object(_)) => Some(object),
        _ => None,
    };
    match outcome {
        Ok(output) => {
            let failed = output.exit_code != 0;
            if !failed
                && output.stderr.is_empty()
                && let Some(payload) = object(&output.stdout)
            {
                return ToolOutcome::ok(payload);
            }
            if failed
                && output.stdout.is_empty()
                && let Some(error) = object(&output.stderr)
            {
                return ToolOutcome::failed(error);
            }
            let failed = output.exit_code != 0;
            let mut blocks = vec![output.stdout];
            if !output.stderr.is_empty() {
                blocks.push(output.stderr);
            }
            ToolOutcome::texts(blocks, failed)
        }
        Err(error) => extension_error(&error.diagnostic()),
    }
}

/// The core tool named `name`.
pub fn core_tool(name: &str) -> Option<&'static ToolSpec> {
    CORE_TOOLS.iter().find(|t| t.name == name)
}

pub fn handle_tool_call(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    // A request that fails CallToolRequest's own schema is malformed: a
    // protocol error (ADR 0004 D4-a).
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: name",
            );
        }
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => Value::Object(Default::default()),
        Some(object @ Value::Object(_)) => object.clone(),
        Some(_) => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Invalid params: arguments must be an object",
            );
        }
    };

    // So is an unknown or disabled tool: it is not an invocation.
    let spec = core_tool(name);
    let extension = match spec {
        Some(_) => None,
        None => match state.surfaces().tool(name) {
            Some(entry) => Some(entry.clone()),
            None => {
                // MCP spec (tools/call): an unrecognized tool is an Invalid
                // params protocol error, as its "Unknown tool" example shows.
                return JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    format!("Unknown tool: {name}"),
                );
            }
        },
    };

    // The category it is listed with: no second lookup.
    let category = match (spec, &extension) {
        (Some(spec), _) => spec.category.as_str(),
        (None, Some(entry)) => entry.category.as_str(),
        (None, None) => unreachable!("an unknown tool was refused above"),
    };
    let mut event = json!({
        "toolName": name,
        "category": category,
        "params": arguments.to_string(),
    });
    if let Some(entity_id) = arguments.get("entity_id").and_then(Value::as_str) {
        event["entityId"] = Value::from(entity_id);
    }
    state.push_event("mcp_tool_invoked", event);

    // The project the call acts on, resolved (and brought up to date)
    // before the handler runs: handlers never pick a root or reload.
    let target_spec = spec.map_or(TargetSpec::SERVED, |spec| spec.target);
    let outcome = match (spec.map(|spec| spec.handler), &extension) {
        // A mutation says what it wrote; `mutation::refresh` brings the
        // target up to date with it (inside the call), `mutation::report`
        // records its events and names the files in its reply.
        (Some(Handler::Mutation(handler)), _) => {
            let (mutated, root) = match target::resolve(state, target_spec, &arguments) {
                Err(refused) => (Mutated::refused(McpError::from(refused)), None),
                Ok(target) => {
                    let mut call = Call::new(state, target);
                    let mut mutated = handler(&mut call, arguments);
                    let root = mutation::refresh(&mut call, &mut mutated);
                    (mutated, root)
                }
            };
            mutation::report(state, name, root.as_deref(), mutated.from_tool(name))
        }
        (Some(Handler::Tool(handler)), _) => {
            match target::resolve(state, target_spec, &arguments) {
                Err(refused) => ToolOutcome::from(McpError::from(refused)),
                Ok(target) => {
                    let mut call = Call::new(state, target);
                    let outcome = handler(&mut call, arguments);
                    target::without_project_outcome(call.target(), outcome)
                }
            }
            .from_tool(name)
        }
        (None, Some(entry)) => match target::resolve(state, target_spec, &arguments) {
            Err(refused) => ToolOutcome::from(McpError::from(refused)),
            Ok(target) => {
                let mut call = Call::new(state, target);
                let (outcome, dispatched) = extension_tool(&mut call, entry, arguments);
                if let Some((event, params)) = dispatched {
                    call.state.push_event(event, params);
                }
                target::without_project_outcome(call.target(), outcome)
            }
        }
        .from_tool(name),
        (None, None) => unreachable!("an unknown tool was refused above"),
    };

    // A tool with an outputSchema: a core one, or an extension's that
    // declares one.
    let typed = match (spec, &extension) {
        (Some(spec), _) => spec.output.is_some(),
        (None, Some(entry)) => entry.output_schema().is_some(),
        (None, None) => false,
    };
    envelope(outcome, id, state.sends_structured_content(), typed)
}

/// A dispatch event: its name and payload.
type Dispatched = Option<(&'static str, Value)>;

/// An extension tool, found in the extension surface table, run by one of
/// its two adapters over the `WasmRuntime` seam the call's project was
/// compiled in: an explicit tool's `mcp__` export, or a command's `cmd__`
/// export. Arguments its declaration refuses are refused first, the
/// project resolved after; the dispatch event is the one to record when
/// the export returned (whatever the result: a schema mismatch is a
/// dispatched tool that failed).
fn extension_tool(
    call: &mut Call<'_>,
    entry: &ToolEntry,
    arguments: Value,
) -> (ToolOutcome, Dispatched) {
    match &entry.kind {
        ToolKind::McpTool {
            export,
            input_schema,
            output_schema,
        } => {
            if let Err(refused) = check_input(input_schema, &arguments) {
                return (refused, None);
            }
            let project = match call.project() {
                Ok(project) => project,
                Err(refused) => return (refused.into(), None),
            };
            mcp_tool_adapter(
                project.runtime.as_ref(),
                entry,
                export,
                output_schema.as_ref(),
                &arguments,
            )
        }
        ToolKind::Command(command) => {
            let given = arguments.as_object().cloned().unwrap_or_default();
            // The args the command line would send for the same input, or
            // the command's own INVALID_INPUT object the CLI writes (D5).
            let args = match command.normalize(&given) {
                Ok(args) => args,
                Err(refused) => return (ToolOutcome::failed(refused.to_json()), None),
            };
            let project = match call.project() {
                Ok(project) => project,
                Err(refused) => return (refused.into(), None),
            };
            command_adapter(
                project.runtime.as_ref(),
                project.graph,
                project.root,
                command,
                &args,
            )
        }
    }
}

/// An explicit tool's arguments checked against the input schema it
/// declares, before its module runs: `invalid_input` naming each
/// violation (its schema is opaque JSON to the host, ADR 0004 D4-a).
fn check_input(schema: &Value, arguments: &Value) -> Result<(), ToolOutcome> {
    let violations = crate::json_schema::violations(schema, arguments);
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

/// How long since `started`, in whole milliseconds.
fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// An explicit tool: its `mcp__` export called with the arguments, its
/// output checked against the output schema it declares.
fn mcp_tool_adapter(
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    entry: &ToolEntry,
    export: &str,
    output_schema: Option<&Value>,
    arguments: &Value,
) -> (ToolOutcome, Dispatched) {
    let started = std::time::Instant::now();
    let result = match specforge_wasm::ExtensionCalls::new(runtime).call_mcp_tool(
        &entry.extension,
        export,
        arguments,
    ) {
        Ok(value) => match output_schema {
            // An output the tool's own schema refuses is never served as
            // its structured result.
            Some(schema) => {
                let violations = crate::json_schema::violations(schema, &value);
                if violations.is_empty() {
                    ToolOutcome::ok(value)
                } else {
                    McpError::new(
                        ErrorCode::SchemaMismatch,
                        format!(
                            "extension tool '{}' returned output that does not match its output schema: {}",
                            entry.name,
                            violations.join("; ")
                        ),
                    )
                    .with_data(json!({ "violations": violations }))
                    .into()
                }
            }
            None => ToolOutcome::ok(value),
        },
        // A failed call is the tool's error, and no dispatch is recorded.
        Err(error) => return (extension_error(&error.diagnostic()), None),
    };
    // A tool whose export returned is a dispatched tool; it succeeded when
    // its output is the tool's result.
    let event = json!({
        "extensionName": entry.extension,
        "toolName": entry.name,
        "durationMs": elapsed_ms(started),
        "success": result.succeeded(),
    });
    (result, Some(("surface_mcp_tool_dispatched", event)))
}

/// An extension command: its `cmd__` export run with `args` (normalized
/// by its derivation) over the call's graph, as `specforge <ext>
/// <command>` runs it over the compiled one, always asked for json: the
/// tool has no format argument (ADR 0011).
fn command_adapter(
    runtime: &dyn specforge_wasm::runtime::WasmRuntime,
    graph: &specforge_graph::Graph,
    root: &std::path::Path,
    command: &specforge_ops::command::ExtensionCommand,
    args: &serde_json::Map<String, Value>,
) -> (ToolOutcome, Dispatched) {
    let context = specforge_ops::command::CommandContext {
        format: specforge_ops::command::CommandFormat::Json,
        today: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    };
    let started = std::time::Instant::now();
    let outcome =
        specforge_ops::command::run_command(runtime, command, graph, args, root, &context);
    // A command whose export returned is a dispatched command; a trap is
    // the tool's error.
    let dispatched = outcome.as_ref().ok().map(|output| {
        let event = json!({
            "extensionName": command.extension(),
            "commandId": command.id(),
            "exitCode": output.exit_code,
            "durationMs": elapsed_ms(started),
        });
        ("surface_command_dispatched", event)
    });
    (command_tool_result(outcome), dispatched)
}
