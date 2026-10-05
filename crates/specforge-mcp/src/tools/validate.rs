use serde::Deserialize;
use std::path::PathBuf;

use crate::args::{lenient, strings};
use crate::state::McpState;
use crate::tool::ToolOutcome;
use specforge_project::DiagnosticPolicy;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    severity_filter: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    strict: Option<bool>,
    #[serde(default, deserialize_with = "strings")]
    lint: Vec<String>,
    #[serde(default, deserialize_with = "lenient")]
    use_cached: Option<bool>,
}

pub fn call(state: &mut McpState, args: Args) -> ToolOutcome {
    let path = args
        .path
        .map(PathBuf::from)
        .or_else(|| state.project_root().map(std::path::Path::to_path_buf));

    let root = match path {
        Some(p) => p,
        None => {
            return ToolOutcome::no_project("No project root available; pass {\"path\": ...}");
        }
    };

    let severity_filter = args.severity_filter.as_deref();
    let use_cached = args.use_cached.unwrap_or(false);

    // A path naming another project is validated for this call only: the
    // server keeps serving its own.
    let reported: Vec<specforge_common::Diagnostic> = if state.serves_other_than(&root) {
        state.compile_project(&root).diagnostics()
    } else {
        if !use_cached || state.diagnostics().is_empty() {
            state.serve(&root);
        }
        state.diagnostics()
    };

    // The policy `specforge check` applies: lint profiles add theirs, and
    // strict promotes warnings before filtering, so a promoted warning
    // counts as an error for `severity_filter` and `isError` alike.
    let policy = DiagnosticPolicy {
        strict: args.strict.unwrap_or(false),
        lint_profiles: args.lint,
    };
    let promoted = policy.apply(&root, reported);
    let diagnostics: Vec<&specforge_common::Diagnostic> = promoted
        .iter()
        .filter(|d| match severity_filter {
            Some("error") => d.severity == specforge_common::Severity::Error,
            Some("warning") => d.severity == specforge_common::Severity::Warning,
            Some("info") => d.severity == specforge_common::Severity::Info,
            _ => true,
        })
        .collect();

    let filtered: Vec<specforge_common::Diagnostic> = diagnostics.into_iter().cloned().collect();
    let diag_json = specforge_common::serialize_diagnostics(&filtered);

    ToolOutcome::text(diag_json)
}
