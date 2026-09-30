use serde_json::Value;
use std::path::PathBuf;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &mut McpState, args: Value) -> ToolOutcome {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let root = match path {
        Some(p) => p,
        None => {
            return ToolOutcome::invalid_params("No project root available");
        }
    };

    let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());
    let use_cached = args
        .get("use_cached")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // A path naming another project is validated for this call only: the
    // server keeps serving its own.
    let reported: Vec<specforge_common::Diagnostic> = if state.serves_other_than(&root) {
        state.compile_project(&root).diagnostics()
    } else {
        if !use_cached || state.diagnostics.is_empty() {
            state.recompile(&root);
        }
        state.diagnostics.clone()
    };

    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Strict promotes warnings before filtering, so a promoted warning
    // counts as an error for `severity_filter` and `isError` alike.
    let promoted: Vec<specforge_common::Diagnostic> = reported
        .into_iter()
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

    ToolOutcome::text(diag_json).flagged(has_errors)
}
