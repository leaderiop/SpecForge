use crate::pipeline;
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct PluginsArgs {
    /// Path to project root (default: current directory)
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
}

/// Run the plugins command. Returns exit code.
pub fn run(args: PluginsArgs) -> i32 {
    let result = match pipeline::run_pipeline(&args.path) {
        Ok(r) => r,
        Err(code) => return code,
    };

    if result.config.plugins.is_empty() {
        println!("No plugins installed.");
        return 0;
    }

    for module in &result.config.plugins {
        if let Some(package) = module.package_name() {
            println!(
                "  {}  ({} entities, {} edges)  [built-in]",
                package,
                module.entity_count(),
                module.edge_count(),
            );
        }
    }

    // Show Wasm plugins
    if let Some(runtime) = result.wasm_pool.runtime() {
        for info in runtime.package_infos() {
            println!(
                "  {}  v{}  [{:?}]  [wasm]",
                info.package, info.version, info.state,
            );
        }
    }

    0
}
