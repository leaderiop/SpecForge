use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct StatsArgs {
    /// Path to check (directory or file). Defaults to current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the stats command — prints project statistics to stdout. Returns exit code.
pub fn run(args: StatsArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let stats =
        specforge_emitter::compute_stats(&result.graph, &result.files, &result.diagnostics);
    let output = specforge_emitter::format_stats(&stats);
    print!("{output}");
    0
}
