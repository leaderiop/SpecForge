use serde_json::Value;
use std::path::PathBuf;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &mut McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let root = match path {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "No project root available",
            );
        }
    };

    let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());
    let use_cached = args
        .get("use_cached")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !use_cached || state.diagnostics.is_empty() {
        state.recompile(&root);
    }

    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Strict promotes warnings before filtering, so a promoted warning
    // counts as an error for `severity_filter` and `isError` alike.
    let promoted: Vec<specforge_common::Diagnostic> = state
        .diagnostics
        .iter()
        .cloned()
        .map(|mut d| {
            if strict && d.severity == specforge_common::Severity::Warning {
                d.severity = specforge_common::Severity::Error;
            }
            d
        })
        .collect();
    let diagnostics: Vec<&specforge_common::Diagnostic> = promoted
        .iter()
        .filter(|d| match severity_filter {
            Some("error") => d.severity == specforge_common::Severity::Error,
            Some("warning") => d.severity == specforge_common::Severity::Warning,
            Some("info") => d.severity == specforge_common::Severity::Info,
            _ => true,
        })
        .collect();

    let has_errors = diagnostics
        .iter()
        .any(|d| d.severity == specforge_common::Severity::Error);
    let filtered: Vec<specforge_common::Diagnostic> = diagnostics.into_iter().cloned().collect();
    let diag_json = specforge_emitter::serialize_diagnostics(&filtered);

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": diag_json
            }],
            "isError": has_errors
        }),
    )
}
