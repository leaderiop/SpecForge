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
pub(crate) mod trace;
mod validate;

use serde_json::{Value, json};
use specforge_registry::SurfaceType;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;
use crate::tool::{ToolOutcome, envelope};

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

/// Whether a mutation tool call changes files, with each tool's defaults:
/// format's check and diff modes and every dry run only report, and
/// report-only calls complete no mutation.
fn writes(name: &str, args: &Value) -> bool {
    let flag = |key: &str| args.get(key).and_then(Value::as_bool);
    match name {
        "specforge.format" => flag("write")
            .unwrap_or(!flag("check").unwrap_or(false) && !flag("diff").unwrap_or(false)),
        _ => !flag("dry_run").unwrap_or(false),
    }
}

/// Tools that change files on disk.
const MUTATION_TOOLS: [&str; 7] = [
    "specforge.format",
    "specforge.rename",
    "specforge.init",
    "specforge.add_extension",
    "specforge.remove_extension",
    "specforge.migrate",
    "specforge.infer_session",
];

/// The `McpToolCategory` of a registered tool (`core`, `navigation`,
/// `mutation` or `management`), or `None` for a tool the server does not
/// know. Tools that write files are mutations; registry categories outside
/// the four (`inference`, `extension`) are read-only analysis, so `core`.
fn tool_category(state: &McpState, name: &str) -> Option<&'static str> {
    let registered = state.tool_registry.iter().find(|t| t.name == name)?;
    if MUTATION_TOOLS.contains(&name) {
        return Some("mutation");
    }
    Some(match registered.category.as_deref() {
        Some("navigation") => "navigation",
        Some("mutation") => "mutation",
        Some("management") => "management",
        _ => "core",
    })
}

/// What a completed mutation changed, read from the tool's own result:
/// `(files_changed, entities_affected)`.
fn mutation_effect(name: &str, outcome: &Value) -> (usize, usize) {
    let count = |key: &str| outcome[key].as_array().map_or(0, Vec::len);
    match name {
        // Every file the formatter rewrote; formatting changes no entity.
        "specforge.format" => (count("changed_files"), 0),
        // The files holding the entity or a reference to it; one entity.
        "specforge.rename" => (count("affected_files"), 1),
        // The project config, the starter spec file and .gitignore.
        "specforge.init" => (3, 0),
        // The extension module, the lock file and the project config.
        "specforge.add_extension" if outcome["installed"] == true => (3, 0),
        // The same three; entities whose kind only it defined lose it.
        "specforge.remove_extension" if outcome["success"] == true => (3, count("orphan_warnings")),
        "specforge.migrate" if outcome["migrated"] == true => {
            (outcome["files_migrated"].as_u64().unwrap_or(0) as usize, 0)
        }
        // specforge-infer.json; mark_analyzed records the entities produced.
        "specforge.infer_session" => (1, count("entities_produced")),
        _ => (0, 0),
    }
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

    // An unknown tool is a protocol error, not an invocation.
    if let Some(category) = tool_category(state, name) {
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

    let is_mutation = MUTATION_TOOLS.contains(&name) && writes(name, &arguments);

    let mut outcome = dispatch(state, name, arguments);
    for (event, params) in outcome.take_events() {
        state.push_event(event, params);
    }

    // A refused call ran nothing; a run reports what its structured
    // result says it changed.
    if is_mutation && !outcome.is_refused() {
        let success = outcome.succeeded();
        let (files_changed, entities_affected) = outcome
            .success_payload()
            .map_or((0, 0), |payload| mutation_effect(name, payload));
        state.push_event(
            "mcp_mutation_completed",
            json!({
                "toolName": name,
                "files_changed": files_changed,
                "entities_affected": entities_affected,
                "success": success,
            }),
        );
    }

    envelope(outcome, id, state.sends_structured_content())
}

/// Run the tool `name` with `arguments`.
fn dispatch(state: &mut McpState, name: &str, arguments: Value) -> ToolOutcome {
    match name {
        // Core tools
        "specforge.query" => query::call(state, arguments),
        "specforge.validate" => validate::call(state, arguments),
        "specforge.export" => export::call(state, arguments),
        "specforge.trace" => trace::call(state, arguments),
        "specforge.search" => search::call(state, arguments),
        "specforge.schema" => schema::call(state, arguments),
        "specforge.model" => model::call(state, arguments),
        "specforge.coverage" => coverage::call(state, arguments),
        "specforge.analyze" => analyze::call(state, arguments),
        "specforge.stats" => stats::call(state, arguments),
        // Navigation tools
        "specforge.list" => list::call(state, arguments),
        "specforge.inspect" => inspect::call(state, arguments),
        "specforge.find_definition" => find_definition::call(state, arguments),
        "specforge.find_references" => find_references::call(state, arguments),
        "specforge.outline" => outline::call(state, arguments),
        "specforge.outline_extensions" => outline_extensions::call(state, arguments),
        "specforge.suggest_fixes" => suggest_fixes::call(state, arguments),
        // Inference tools
        "specforge.infer_progress" => infer_progress::call(state, arguments),
        "specforge.infer_session" => infer_session::call(state, arguments),
        "specforge.infer_gaps" => infer_gaps::call(state, arguments),
        // Source anchoring tools
        "specforge.find_implementation" => find_implementation::call(state, arguments),
        "specforge.find_spec_for_source" => find_spec_for_source::call(state, arguments),
        // Operations
        "specforge.format"
        | "specforge.rename"
        | "specforge.init"
        | "specforge.add_extension"
        | "specforge.remove_extension"
        | "specforge.migrate"
        | "specforge.extensions"
        | "specforge.providers"
        | "specforge.doctor"
        | "specforge.collect"
        | "specforge.render" => crate::operations::handle_operation(state, name, arguments, None),
        _ => extension_tool(state, name, arguments),
    }
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
