use serde::Deserialize;
use std::path::PathBuf;

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::ToolOutcome;
use specforge_ops::analyze::{AnalyzeError, AnalyzeOptions, ProjectView, ReportSource, analyze};
use specforge_project::CompiledProject;
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};

/// The runtime of a rootless analysis, which runs no extension pass.
struct NoRuntime;

impl WasmRuntime for NoRuntime {
    fn load_module(&self, _: &std::path::Path) -> Result<(), String> {
        Err("no project root".to_string())
    }

    fn call_export(&self, extension: &str, export: &str, _: &[u8]) -> WasmCallResult {
        WasmCallResult::Trap(specforge_wasm::runtime::WasmTrapInfo {
            kind: "export_not_found".to_string(),
            message: format!("{extension}: no project root"),
            export_name: export.to_string(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    pass: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    strict: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    test_results: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    use_cached: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    path: Option<String>,
}

/// `specforge.analyze` — run the analysis passes (coverage, contracts) plus
/// extension-owned compiler passes over the project and return structured
/// findings. Extension passes execute through the same Wasm runtime the CLI
/// uses (WASM-only migration, Phase 4).
pub fn call(state: &mut McpState, args: Args) -> ToolOutcome {
    let path = args
        .path
        .map(PathBuf::from)
        .or_else(|| state.project_root.clone());

    let use_cached = args.use_cached.unwrap_or(false);

    // A path naming another project is analyzed for this call only: the
    // server keeps serving its own. Otherwise the served project is
    // recompiled unless the caller opted into the cached graph; a rootless
    // server analyzes its existing state.
    let other: Option<(PathBuf, CompiledProject)> = match &path {
        Some(root) if state.serves_other_than(root) => {
            Some((root.clone(), state.compile_project(root)))
        }
        Some(root) => {
            if !use_cached || state.graph().node_count() == 0 {
                state.reload(root);
            }
            None
        }
        None => None,
    };
    let state: &McpState = state;
    let view = match &other {
        Some((root, project)) => {
            ProjectView::in_environment(&project.env, &project.graph, Some(root.as_path()))
        }
        None => state.project_view(),
    };
    // Without `test_results`, use what `specforge collect` last recorded, as
    // the CLI does. Extension passes need the Wasm runtime, only when a root
    // is known.
    let options = AnalyzeOptions {
        pass: args.pass.unwrap_or_else(|| "all".to_string()),
        strict: args.strict.unwrap_or(false),
        report: match args.test_results {
            Some(named) => ReportSource::File(PathBuf::from(named)),
            None => ReportSource::RecordedInRoot,
        },
        min: None,
        prove: None,
    };
    let runtime = match view.root {
        Some(root) => state.wasm_runtime(root),
        None => std::sync::Arc::new(NoRuntime),
    };
    match analyze(&view, runtime.as_ref(), &options) {
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
    }
}
