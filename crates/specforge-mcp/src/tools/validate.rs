use serde_json::Value;
use std::path::PathBuf;

use crate::compile::compile_project;
use crate::protocol::{JsonRpcResponse, error_codes};
use crate::registry::register_extension_surfaces;
use crate::state::McpState;

pub fn call(state: &mut McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let path = args.get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let root = match path {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "No project root available"),
    };

    let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());
    let use_cached = args.get("use_cached").and_then(|v| v.as_bool()).unwrap_or(false);

    if !use_cached || state.diagnostics.is_empty() {
        let result = compile_project(&root);
        state.graph = result.graph;
        state.diagnostics = result.diagnostics;
        state.kind_registry = result.kind_registry;
        state.field_registry = result.field_registry;
        state.edge_registry = result.edge_registry;
        state.extension_info = result.extension_info;
        state.surface_entries = result.surface_entries;

        // Re-register extension surfaces (remove old extension tools/resources first)
        state.tool_registry.retain(|t| {
            t.category.as_deref() != Some("extension")
        });
        state.resource_registry.retain(|r| {
            // Keep core resources, remove extension-added ones
            r.uri.starts_with("specforge://")
                && !r.uri.starts_with("specforge://ext/")
        });
        register_extension_surfaces(state, &result.manifest_surfaces);
    }

    let diagnostics: Vec<&specforge_common::Diagnostic> = state.diagnostics.iter()
        .filter(|d| {
            match severity_filter {
                Some("error") => d.severity == specforge_common::Severity::Error,
                Some("warning") => d.severity == specforge_common::Severity::Warning,
                Some("info") => d.severity == specforge_common::Severity::Info,
                _ => true,
            }
        })
        .collect();

    let has_errors = diagnostics.iter().any(|d| d.severity == specforge_common::Severity::Error);
    let filtered: Vec<specforge_common::Diagnostic> = diagnostics.into_iter().cloned().collect();
    let diag_json = specforge_emitter::serialize_diagnostics(&filtered);

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": diag_json
        }],
        "isError": has_errors
    }))
}
