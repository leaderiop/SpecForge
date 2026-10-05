use serde::Deserialize;

use crate::args::{lenient, strings};
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};
use specforge_project::DiagnosticPolicy;

#[derive(Debug, Deserialize)]
pub struct Args {
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    severity_filter: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    strict: Option<bool>,
    #[serde(default, deserialize_with = "strings")]
    lint: Vec<String>,
    /// Read by the call's target (`Freshness::FreshUnlessCached`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target applies use_cached")]
    use_cached: Option<bool>,
}

/// `specforge.validate`: what `specforge check` reports for the call's
/// project. The target brought the served project up to date with disk
/// (unless `use_cached`), or compiled the project `path` names for this
/// call.
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let project = call.project()?;
    let severity_filter = args.severity_filter.as_deref();

    // The policy `specforge check` applies: lint profiles add theirs, and
    // strict promotes warnings before filtering, so a promoted warning
    // counts as an error for `severity_filter` and `isError` alike.
    let policy = DiagnosticPolicy {
        strict: args.strict.unwrap_or(false),
        lint_profiles: args.lint,
    };
    let promoted = policy.apply(project.root, project.diagnostics());
    let filtered: Vec<specforge_common::Diagnostic> = promoted
        .into_iter()
        .filter(|d| match severity_filter {
            Some("error") => d.severity == specforge_common::Severity::Error,
            Some("warning") => d.severity == specforge_common::Severity::Warning,
            Some("info") => d.severity == specforge_common::Severity::Info,
            _ => true,
        })
        .collect();
    let diag_json = specforge_common::serialize_diagnostics(&filtered);

    Ok(ToolOutcome::text(diag_json))
}
