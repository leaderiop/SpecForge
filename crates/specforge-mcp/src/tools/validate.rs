use serde::Deserialize;

use crate::args::{lenient, strings};
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};
use specforge_ops::check::{CheckError, CheckOptions, check, parse_lint_profiles, parse_severity};

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

/// The `_meta` key of validate's verdict: `{ok, errors, warnings, infos,
/// shown}` over everything reported, whatever `severity_filter` shows.
pub const VERDICT_META: &str = "specforge/check";

/// `specforge.validate`: the check operation over what `specforge check`
/// reports for the call's project (the target brought the served project
/// up to date with disk unless `use_cached`, or compiled the project
/// `path` names for this call). Finding errors is a successful call (ADR
/// 0004 D4-a); whether the check passed is the `_meta` verdict. The tool
/// never records the build cache.
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let severity = match args.severity_filter.as_deref().map(parse_severity) {
        None => None,
        Some(Ok(severity)) => Some(severity),
        Some(Err(error)) => return Ok(refused(error, "severity_filter")),
    };
    let lint_profiles = match parse_lint_profiles(&args.lint) {
        Ok(profiles) => profiles,
        Err(error) => return Ok(refused(error, "lint")),
    };
    let project = call.project()?;
    let options = CheckOptions {
        strict: args.strict.unwrap_or(false),
        lint_profiles,
        severity,
        record_cache: false,
    };
    let outcome = match check(&project.view(), project.diagnostics(), &options) {
        Ok(outcome) => outcome,
        Err(error) => return Ok(refused(error, "path")),
    };
    let shown: Vec<specforge_common::Diagnostic> = outcome.shown().into_iter().cloned().collect();
    Ok(
        ToolOutcome::text(specforge_common::serialize_diagnostics(&shown))
            .with_meta(VERDICT_META, outcome.verdict_json()),
    )
}

/// Why validate could not run: an argument it cannot use (`invalid_input`
/// naming it, with the closest valid name), or no project root.
fn refused(error: CheckError, argument: &str) -> ToolOutcome {
    let error = specforge_ops::OpError::from(error);
    let argument = (error.kind == specforge_ops::OpErrorKind::InvalidInput).then_some(argument);
    let refused = crate::tool::McpError::from(error);
    match argument {
        Some(argument) => refused.with_argument(argument),
        None => refused,
    }
    .into()
}
