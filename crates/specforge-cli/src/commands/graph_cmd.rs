use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct GraphArgs {
    /// Path to check (directory or file). Defaults to current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// Run the graph command — prints DOT to stdout. Returns exit code.
pub fn run(args: GraphArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let generated = specforge_emitter::render_dot(&result.graph);
    print!("{}", generated.content);
    0
}
