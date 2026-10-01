mod analyze;
pub(crate) mod coverage;
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
use specforge_registry::SurfaceType;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::tool::{Effect, ToolOutcome, ToolSpec, envelope};
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
        .kind_registry
        .keywords()
        .map(String::as_str)
        .chain(state.graph.nodes().into_iter().map(|n| n.kind.raw.as_str()))
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

/// A failed extension call as a failed tool result.
fn extension_error(diag: &specforge_common::Diagnostic) -> ToolOutcome {
    ToolOutcome::failed(format!("{}: {}", diag.code, diag.message))
}

/// An auto-promoted command's run as a tool result: its stdout, then its
/// stderr when it wrote any; a nonzero exit code fails the call.
fn command_tool_result(
    outcome: Result<specforge_wasm::CommandOutput, specforge_common::Diagnostic>,
) -> ToolOutcome {
    match outcome {
        Ok(output) => {
            let mut blocks = vec![String::from_utf8_lossy(&output.stdout).into_owned()];
            if !output.stderr.is_empty() {
                blocks.push(String::from_utf8_lossy(&output.stderr).into_owned());
            }
            ToolOutcome::texts(blocks, output.exit_code != 0)
        }
        Err(diag) => extension_error(&diag),
    }
}

/// The `McpToolCategory` an extension tool's invocation reports: its
/// registered category when it is one of the four, else `core`. `None` for
/// a tool the server does not know.
fn extension_category(state: &McpState, name: &str) -> Option<&'static str> {
    let registered = state.tool_registry.iter().find(|t| t.name == name)?;
    Some(match registered.category.as_deref() {
        Some("navigation") => "navigation",
        Some("mutation") => "mutation",
        Some("management") => "management",
        _ => "core",
    })
}

/// The core tool named `name`.
pub fn core_tool(name: &str) -> Option<&'static ToolSpec> {
    CORE_TOOLS.iter().find(|t| t.name == name)
}

pub fn handle_tool_call(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

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

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let spec = core_tool(name);
    // An unknown tool is a protocol error, not an invocation.
    let category = match spec {
        Some(spec) => Some(spec.event_category()),
        None => extension_category(state, name),
    };
    if let Some(category) = category {
        let mut event = json!({
            "toolName": name,
            "category": category,
            "params": arguments.to_string(),
        });
        if let Some(entity_id) = arguments.get("entity_id").and_then(Value::as_str) {
            event["entityId"] = Value::from(entity_id);
        }
        state.push_event("mcp_tool_invoked", event);
    }

    let mutation = spec
        .and_then(|spec| spec.mutation)
        .filter(|mutation| (mutation.writes)(&arguments));
    let served_since = state.loaded_at;

    let mut outcome = match spec {
        Some(spec) => (spec.call)(state, arguments),
        None => extension_tool(state, name, arguments),
    };
    for (event, params) in outcome.take_events() {
        state.push_event(event, params);
    }

    if let Some(mutation) = mutation {
        // A mutation that wrote files leaves the server serving what is
        // on disk: the tool recompiled already (rename), or it is
        // recompiled now.
        if mutation.recompiles
            && outcome.succeeded()
            && state.loaded_at == served_since
            && let Some(root) = state.project_root.clone()
        {
            state.recompile(&root);
        }
        // A refused call ran nothing; a run reports what its structured
        // result says it changed.
        if !outcome.is_refused() {
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
    }

    envelope(outcome, id, state.sends_structured_content())
}

/// A registered extension tool from surface contributions, run through the
/// Wasm runtime (WASM-only migration, Phase 4).
fn extension_tool(state: &McpState, name: &str, arguments: Value) -> ToolOutcome {
    let Some(entry) = state.surface_entries.iter().find(|e| {
        (e.surface_type == SurfaceType::McpTool || e.surface_type == SurfaceType::AutoPromotedTool)
            && e.contribution_name == name
            && e.enabled
    }) else {
        // MCP spec (tools/call): an unrecognized tool is an Invalid params
        // protocol error — see the "Unknown tool" example in
        // docs/mcp-specification-summary.md.
        return ToolOutcome::invalid_params(format!("Unknown tool: {}", name));
    };
    let Some(root) = state.project_root.clone() else {
        return ToolOutcome::invalid_params(format!(
            "Extension tool '{}' needs a project root; pass {{\"path\": ...}} to specforge.analyze first",
            name
        ));
    };
    let runtime = state.wasm_runtime(&root);
    let input = serde_json::to_vec(&arguments).unwrap_or_default();
    if entry.surface_type == SurfaceType::AutoPromotedTool {
        // An auto-promoted CLI command runs its cmd__ export.
        return command_tool_result(specforge_wasm::dispatch_surface_command(
            &entry.extension_name,
            &entry.export_name,
            &input,
            runtime.as_ref(),
        ));
    }
    match specforge_wasm::dispatch_surface_mcp_tool(
        &entry.extension_name,
        &entry.export_name,
        &input,
        runtime.as_ref(),
    ) {
        Ok(value) => ToolOutcome::ok(value),
        Err(diag) => extension_error(&diag),
    }
}
