use serde::Deserialize;
use std::path::PathBuf;

use crate::args::lenient;
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};
use specforge_ops::analyze::{AnalyzeError, AnalyzeOptions, ReportSource, analyze};
use specforge_wasm::runtime::WasmRuntime;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    pass: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    strict: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    test_results: Option<String>,
    /// Read by the call's target (`Freshness::FreshUnlessCached`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target applies use_cached")]
    use_cached: Option<bool>,
    /// Read by the call's target (`target::resolve`), not here.
    #[serde(default, deserialize_with = "lenient")]
    #[allow(dead_code, reason = "the call target resolves path")]
    path: Option<String>,
}

/// `specforge.analyze` — run the analysis passes (coverage, contracts) plus
/// extension-owned compiler passes over the call's project and return
/// structured findings. Extension passes execute in the runtime the
/// project was compiled in (WASM-only migration, Phase 4): the served
/// session's, or the one runtime another project was compiled in for this
/// call. With no project served and no `path`, there is nothing to
/// analyze: a no-project refusal (plan 01 D7).
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let project = call.project()?;
    let view = project.view();
    // Without `test_results`, use what `specforge collect` last recorded at
    // the project root, as the CLI does.
    let options = AnalyzeOptions {
        pass: args.pass.unwrap_or_else(|| "all".to_string()),
        strict: args.strict.unwrap_or(false),
        report: match args.test_results {
            Some(named) => ReportSource::File(PathBuf::from(named)),
            None => ReportSource::Recorded,
        },
        min: None,
        prove: None,
    };
    // A project built in memory may have no runtime: then no extension
    // pass runs (plan 04 D13).
    let runtime: Option<&dyn WasmRuntime> = project.runtime.map(|runtime| runtime.as_ref() as _);
    Ok(match analyze(&view, runtime, &options) {
        Ok(outcome) => ToolOutcome::ok(outcome.to_json()),
        Err(e @ AnalyzeError::UnknownPass { .. }) => {
            ToolOutcome::invalid_input("pass", e.to_string())
        }
        Err(AnalyzeError::UnusableReport(e)) => {
            let mut error = crate::operations::op_error(e);
            error.tool = Some("specforge.analyze".to_string());
            error.into()
        }
        // `min` is never set here, so this is not reached today.
        Err(e) => ToolOutcome::invalid_input("test_results", e.to_string()),
    })
}
