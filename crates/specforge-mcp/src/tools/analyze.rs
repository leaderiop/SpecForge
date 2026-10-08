use crate::args::Arguments;
use crate::target::Call;
use crate::tool::{Handled, McpError, ToolOutcome};
use specforge_ops::OpError;
use specforge_ops::analyze::{AnalyzeError, AnalyzeOptions, ReportSource, analyze};
use specforge_wasm::runtime::WasmRuntime;

/// `specforge.analyze`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Analysis pass to run: all, coverage, contracts, or a pass an extension declares (`<extension>:<pass>`)
    #[arg(default = specforge_ops::analyze::EVERY_PASS.to_string())]
    pass: String,
    /// Promote warnings to errors
    strict: bool,
    /// Path to a specforge-report.json for proof-level verdicts
    test_results: Option<String>,
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
        pass: args.pass,
        strict: args.strict,
        report: match args.test_results {
            // A relative path names a file under the call's project, as the
            // paths of `specforge.format` do; an absolute one is itself.
            Some(named) => ReportSource::File(project.root.join(named)),
            None => ReportSource::Recorded,
        },
        min: None,
        prove: None,
    };
    // The project's runtime: every project a call reaches has one, so its
    // extensions' passes run (ADR 0017).
    let runtime: Option<&dyn WasmRuntime> = Some(project.runtime.as_ref());
    Ok(match analyze(&view, runtime, &options) {
        Ok(outcome) => ToolOutcome::ok(outcome.to_json()),
        Err(e) => {
            // The argument an unknown pass names is this surface's spelling.
            let unknown_pass = matches!(e, AnalyzeError::UnknownPass { .. });
            let error = McpError::from(OpError::from(e));
            match unknown_pass {
                true => error.with_argument("pass"),
                false => error,
            }
            .into()
        }
    })
}
