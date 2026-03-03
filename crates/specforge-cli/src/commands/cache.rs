use clap::{Args, Subcommand};
use std::path::PathBuf;

use specforge_wasm::cache::AotCache;

#[derive(Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Show cache status
    Status {
        /// Path to project root (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Clear all cached AOT artifacts
    Clear {
        /// Path to project root (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}

pub fn run(args: CacheArgs) -> i32 {
    match args.command {
        CacheCommand::Status { path } => run_status(&path),
        CacheCommand::Clear { path } => run_clear(&path),
    }
}

fn run_status(path: &PathBuf) -> i32 {
    let cache = AotCache::for_project(path);
    let status = cache.status();

    eprintln!("specforge cache status\n");
    eprintln!("  Cache dir:       {}", status.cache_dir.display());
    eprintln!("  Entries:         {}", status.entry_count);
    eprintln!("  Total size:      {}", status.size_display());
    eprintln!("  Runtime version: {}", status.runtime_version);

    0
}

fn run_clear(path: &PathBuf) -> i32 {
    let cache = AotCache::for_project(path);

    match cache.clear() {
        Ok(count) => {
            eprintln!("specforge: cleared {count} cache entries");
            0
        }
        Err(e) => {
            eprintln!("specforge: error clearing cache: {e}");
            1
        }
    }
}
