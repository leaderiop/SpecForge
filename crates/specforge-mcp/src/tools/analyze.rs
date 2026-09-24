use serde_json::Value;
use std::path::PathBuf;

use crate::compile::compile_project;
use crate::protocol::{JsonRpcResponse, error_codes};
use crate::registry::register_extension_surfaces;
use crate::state::McpState;
use specforge_emitter::analyze::{AnalysisContext, TestReport, run_pass};

/// `specforge.analyze` — run the built-in analysis passes (coverage,
/// contracts) over the project and return structured findings.
///
/// Extension-owned compiler passes are not dispatched here: they require a
/// Wasm runtime for execution. Use `specforge analyze` from the CLI for the
/// full pass set.
pub fn call(state: &mut McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let use_cached = args
        .get("use_cached")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Recompile only when a root is available and the caller did not opt
    // into the cached graph; a rootless server analyzes its existing state.
    if let Some(root) = &path
        && (!use_cached || state.graph.node_count() == 0)
    {
        let result = compile_project(root);
        state.graph = result.graph;
        state.diagnostics = result.diagnostics;
        state.kind_registry = result.kind_registry;
        state.field_registry = result.field_registry;
        state.edge_registry = result.edge_registry;
        state.extension_info = result.extension_info;
        state.surface_entries = result.surface_entries;
        state.manifests = result.manifests;
        state.project_root = Some(root.clone());

        state
            .tool_registry
            .retain(|t| t.category.as_deref() != Some("extension"));
        state.resource_registry.retain(|r| {
            r.uri.starts_with("specforge://") && !r.uri.starts_with("specforge://ext/")
        });
        register_extension_surfaces(state, &result.manifest_surfaces);
    }

    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let parsed_report = match args.get("test_results").and_then(|v| v.as_str()) {
        Some(report_path) => match std::fs::read_to_string(report_path)
            .map_err(|e| e.to_string())
            .and_then(|raw| {
                serde_json::from_str::<TestReport>(&raw)
                    .map_err(|e| format!("invalid test results: {e}"))
            }) {
            Ok(report) => Some(report),
            Err(e) => {
                return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, e);
            }
        },
        None => None,
    };

    let context = AnalysisContext {
        graph: &state.graph,
        kind_registry: &state.kind_registry,
        field_registry: &state.field_registry,
        project_root: state.project_root.as_deref(),
        test_results: parsed_report.as_ref(),
    };

    let requested = args
        .get("pass")
        .and_then(|v| v.as_str())
        .unwrap_or("all")
        .to_string();
    let pass_names: Vec<&str> = if requested == "all" {
        specforge_emitter::analyze::PASS_NAMES.to_vec()
    } else if specforge_emitter::analyze::PASS_NAMES.contains(&requested.as_str()) {
        vec![requested.as_str()]
    } else {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!(
                "Unknown analysis pass '{requested}' (available: all, {})",
                specforge_emitter::analyze::PASS_NAMES.join(", ")
            ),
        );
    };

    let mut passes = Vec::new();
    let mut has_errors = false;
    for name in pass_names {
        let Some(mut report) = run_pass(&context, name) else {
            continue;
        };
        if strict {
            for d in &mut report.findings {
                if d.severity == specforge_common::Severity::Warning {
                    d.severity = specforge_common::Severity::Error;
                }
            }
        }
        if report
            .findings
            .iter()
            .any(|d| d.severity == specforge_common::Severity::Error)
        {
            has_errors = true;
        }
        passes.push(serde_json::json!({
            "pass": report.name,
            "findings": report.findings,
            "summary": report.summary,
        }));
    }

    let doc = serde_json::json!({ "ok": !has_errors, "passes": passes });
    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&doc).unwrap_or_default()
            }],
            "isError": has_errors
        }),
    )
}
