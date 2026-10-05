mod analyze;
pub(crate) mod coverage;
mod explain;
mod export;
mod find_definition;
mod find_implementation;
mod find_references;
mod find_spec_for_source;
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
use specforge_registry::{SurfaceRegistryEntry, SurfaceType};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::tool::{Category, Effect, ErrorCode, McpError, ToolOutcome, ToolSpec, envelope};
pub use table::CORE_TOOLS;

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
/// missing entity is `entity_not_found`, its `E003` in `diagnostic`, never
/// only in the message text.
fn emitter_error(error: specforge_emitter::EmitterError, entity_id: &str) -> ToolOutcome {
    use specforge_emitter::EmitterError;
    let mcp_error = match &error {
        EmitterError::EntityNotFound(message) => {
            McpError::from_coded_message(ErrorCode::EntityNotFound, message).with_entity(entity_id)
        }
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
    let code = if message.starts_with("failed to read") {
        ErrorCode::InternalError
    } else {
        ErrorCode::SchemaMismatch
    };
    ToolOutcome::error(code, message)
}

/// A failed extension call as a failed tool result: the diagnostic the
/// runtime reported, in `diagnostic`.
fn extension_error(diag: &specforge_common::Diagnostic) -> ToolOutcome {
    McpError::from_diagnostic(diag).into()
}

/// The id of the command an auto-promoted tool runs: the one its extension
/// declares with the tool's export.
fn command_id(state: &McpState, entry: &SurfaceRegistryEntry) -> String {
    state
        .registries()
        .declaration(&entry.extension_name)
        .and_then(|declaration| {
            declaration
                .surfaces
                .commands
                .iter()
                .find(|command| command.export == entry.export_name)
        })
        .map_or_else(|| entry.contribution_name.clone(), |c| c.id.clone())
}

/// An auto-promoted command's run as a tool result. The command was asked
/// for json (ADR 0011): a JSON object on stdout, and nothing on stderr, is
/// the result's structured payload; a failure that wrote one JSON object
/// on stderr, and nothing on stdout, is an `isError` result carrying it.
/// Otherwise its stdout, then its stderr when it wrote any; a nonzero exit
/// code fails the call.
fn command_tool_result(
    outcome: Result<specforge_wasm::CommandOutput, specforge_common::Diagnostic>,
) -> ToolOutcome {
    let object = |bytes: &[u8]| match serde_json::from_slice::<Value>(bytes) {
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
            let mut blocks = vec![String::from_utf8_lossy(&output.stdout).into_owned()];
            if !output.stderr.is_empty() {
                blocks.push(String::from_utf8_lossy(&output.stderr).into_owned());
            }
            ToolOutcome::texts(blocks, output.exit_code != 0)
        }
        Err(diag) => extension_error(&diag),
    }
}

/// The `McpToolCategory` an extension tool's invocation reports: the one
/// it is listed with.
fn extension_category(state: &McpState, name: &str) -> &'static str {
    state
        .tool_registry
        .iter()
        .find(|t| t.name == name)
        .and_then(|t| t.category.as_deref())
        .and_then(Category::parse)
        .unwrap_or(Category::Core)
        .as_str()
}

/// The core tool named `name`.
pub fn core_tool(name: &str) -> Option<&'static ToolSpec> {
    CORE_TOOLS.iter().find(|t| t.name == name)
}

/// The extension tool named `name`.
fn extension_entry(state: &McpState, name: &str) -> Option<SurfaceRegistryEntry> {
    state
        .surface_entries()
        .find(|e| {
            (e.surface_type == SurfaceType::McpTool
                || e.surface_type == SurfaceType::AutoPromotedTool)
                && e.contribution_name == name
        })
        .cloned()
}

pub fn handle_tool_call(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

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
        None => match extension_entry(state, name) {
            Some(entry) => Some(entry),
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

    let category = match spec {
        Some(spec) => spec.category.as_str(),
        None => extension_category(state, name),
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

    let mutation = spec
        .and_then(|spec| spec.mutation)
        .filter(|mutation| (mutation.writes)(&arguments));
    let served_since = state.loaded_at;

    let mut outcome = match (spec, extension) {
        (Some(spec), _) => (spec.call)(state, arguments),
        (None, Some(entry)) => {
            let (outcome, dispatched) = extension_tool(state, &entry, arguments);
            if let Some((event, params)) = dispatched {
                state.push_event(event, params);
            }
            outcome
        }
        (None, None) => unreachable!("an unknown tool was refused above"),
    }
    .from_tool(name);
    for (event, params) in outcome.take_events() {
        state.push_event(event, params);
    }

    if let Some(mutation) = mutation {
        // A mutation that wrote files leaves the server serving what is
        // on disk: the tool updated the served project already (rename),
        // or it is reloaded now.
        if mutation.recompiles
            && outcome.succeeded()
            && state.loaded_at == served_since
            && let Some(root) = state.project_root.clone()
        {
            state.reload(&root);
        }
        // Every call that meant to write reports what its structured
        // result says it changed: nothing, when it failed.
        let effect = outcome
            .success_payload()
            .map_or_else(Effect::default, |payload| (mutation.effect)(payload));
        state.push_event(
            "mcp_mutation_completed",
            json!({
                "toolName": name,
                "files_changed": effect.files_changed,
                "entities_affected": effect.entities_affected,
                "success": outcome.succeeded(),
            }),
        );
    }

    // A tool with an outputSchema: a core one, or an extension's that
    // declares one.
    let typed = match spec {
        Some(spec) => spec.output.is_some(),
        None => state
            .tool_registry
            .iter()
            .any(|t| t.name == name && t.output_schema.is_some()),
    };
    envelope(outcome, id, state.sends_structured_content(), typed)
}

/// A dispatch event: its name and payload.
type Dispatched = Option<(&'static str, Value)>;

/// A registered extension tool from surface contributions, run through the
/// Wasm runtime (WASM-only migration, Phase 4), and the dispatch event to
/// record when its export returned (whatever the result: a schema mismatch
/// is a dispatched tool that failed).
fn extension_tool(
    state: &McpState,
    entry: &SurfaceRegistryEntry,
    arguments: Value,
) -> (ToolOutcome, Dispatched) {
    let Some(root) = state.project_root.clone() else {
        let refused = ToolOutcome::no_project(format!(
            "Extension tool '{}' needs a project root; pass {{\"path\": ...}} to specforge.analyze first",
            entry.contribution_name
        ));
        return (refused, None);
    };
    let declared = state
        .tool_registry
        .iter()
        .find(|t| t.name == entry.contribution_name);
    // The input the tool declares, checked before its module runs.
    if let Some(schema) = declared.map(|t| &t.input_schema) {
        let violations = crate::json_schema::violations(schema, &arguments);
        if !violations.is_empty() {
            let refused = McpError::new(
                ErrorCode::InvalidInput,
                format!(
                    "the arguments do not match the tool's input schema: {}",
                    violations.join("; ")
                ),
            )
            .with_data(json!({ "violations": violations }))
            .into();
            return (refused, None);
        }
    }
    let runtime = state.wasm_runtime(&root);
    if entry.surface_type == SurfaceType::AutoPromotedTool {
        // An auto-promoted CLI command runs its cmd__ export over the served
        // graph, as `specforge <ext> <command>` does over the compiled one.
        let args = arguments.as_object().cloned().unwrap_or_default();
        // Over MCP a command is always asked for json: the tool has no
        // format argument (ADR 0011).
        let context = specforge_ops::command::CommandContext {
            format: specforge_ops::command::CommandFormat::Json,
            today: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        };
        let started = std::time::Instant::now();
        let outcome = specforge_ops::command::run_command(
            runtime.as_ref(),
            &entry.extension_name,
            &entry.export_name,
            state.graph(),
            &args,
            &root,
            &context,
        );
        // A command whose export returned is a dispatched command; a trap
        // is the tool's error.
        let dispatched = outcome.as_ref().ok().map(|output| {
            json!({
                "extensionName": entry.extension_name,
                "commandId": command_id(state, entry),
                "exitCode": output.exit_code,
                "durationMs": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            })
        });
        return (
            command_tool_result(outcome),
            dispatched.map(|event| ("surface_command_dispatched", event)),
        );
    }
    let input = serde_json::to_vec(&arguments).unwrap_or_default();
    let started = std::time::Instant::now();
    let result = match specforge_wasm::dispatch_surface_mcp_tool(
        &entry.extension_name,
        &entry.export_name,
        &input,
        runtime.as_ref(),
    ) {
        Ok(value) => match declared.and_then(|t| t.output_schema.as_ref()) {
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
                            entry.contribution_name,
                            violations.join("; ")
                        ),
                    )
                    .with_data(json!({ "violations": violations }))
                    .into()
                }
            }
            None => ToolOutcome::ok(value),
        },
        // A trap is the tool's error, and no dispatch is recorded.
        Err(diag) => return (extension_error(&diag), None),
    };
    // A tool whose export returned is a dispatched tool; it succeeded when
    // its output is the tool's result.
    let event = json!({
        "extensionName": entry.extension_name,
        "toolName": entry.contribution_name,
        "durationMs": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "success": result.succeeded(),
    });
    (result, Some(("surface_mcp_tool_dispatched", event)))
}
