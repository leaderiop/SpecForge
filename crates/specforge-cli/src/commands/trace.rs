use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct TraceArgs {
    /// Entity ID to trace. Omit for full traceability report.
    pub entity_id: Option<String>,

    /// Path to check (directory or file). Defaults to current directory.
    #[arg(long, default_value = ".")]
    pub path: PathBuf,

    /// Enrich trace output with test results from specforge-report.json files.
    #[arg(long)]
    pub test_results: bool,
}

/// Run the trace command. Returns exit code.
pub fn run(args: TraceArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    if args.test_results {
        return run_with_test_results(&result, &args);
    }

    match args.entity_id {
        Some(ref entity_id) => {
            match specforge_emitter::compute_trace(&result.graph, entity_id) {
                Some(chain) => {
                    let output = specforge_emitter::format_trace(&chain);
                    print!("{output}");
                    0
                }
                None => {
                    eprintln!("specforge: entity not found: {entity_id}");
                    1
                }
            }
        }
        None => {
            let report = specforge_emitter::compute_full_trace(&result.graph);
            let output = specforge_emitter::format_trace_report(&report);
            print!("{output}");
            0
        }
    }
}

fn run_with_test_results(result: &pipeline::PipelineResult, args: &TraceArgs) -> i32 {
    let project_root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    let merged =
        specforge_coverage::discover_and_merge(&result.config.coverage.test_dirs, &project_root);

    let summary =
        specforge_coverage::compute_coverage(&result.graph, &result.files, &merged);

    let output = specforge_coverage::render_traceability_matrix(&summary);
    print!("{output}");
    0
}
