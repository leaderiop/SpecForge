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
}

/// Run the trace command. Returns exit code.
pub fn run(args: TraceArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

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
