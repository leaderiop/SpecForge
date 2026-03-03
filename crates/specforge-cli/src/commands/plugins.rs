use crate::pipeline;
use clap::Args;
use specforge_common::PluginManifest;
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
        if let Some(manifest) = PluginManifest::for_module(*module) {
            println!(
                "  {}  ({} entities, {} edges)",
                manifest.package,
                manifest.entity_count(),
                manifest.edge_count(),
            );
        }
    }

    0
}
