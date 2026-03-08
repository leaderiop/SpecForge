mod check;
mod export;
mod query;
mod stats;
mod trace;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "specforge", version, about = "SpecForge compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate .spec files and report diagnostics
    Check {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Promote warnings to errors
        #[arg(long)]
        strict: bool,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Export spec graph to stdout in various formats
    Export {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: graph, brief, context, or dot
        #[arg(long, default_value = "graph")]
        format: String,

        /// Scope export to subgraph reachable from this entity ID
        #[arg(long)]
        scope: Option<String>,
    },
    /// Query the graph at multiple resolutions
    Query {
        /// Entity ID to query
        entity: String,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Number of hops from the entity (0 = entity only)
        #[arg(long, default_value = "1")]
        depth: usize,

        /// Filter results to specific entity kinds (can be repeated)
        #[arg(long)]
        kind: Vec<String>,
    },
    /// Show traceability chain for an entity
    Trace {
        /// Entity ID to trace
        entity: String,

        /// Path to the spec root directory
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Show project statistics
    Stats {
        /// Path to the spec root directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: human or json
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { path, strict, format } => {
            let exit_code = check::run(&path, strict, &format);
            std::process::exit(exit_code);
        }
        Commands::Export { path, format, scope } => {
            let exit_code = export::run(&path, &format, scope.as_deref());
            std::process::exit(exit_code);
        }
        Commands::Query { entity, path, depth, kind } => {
            let exit_code = query::run(&path, &entity, depth, &kind);
            std::process::exit(exit_code);
        }
        Commands::Trace { entity, path } => {
            let exit_code = trace::run(&path, &entity);
            std::process::exit(exit_code);
        }
        Commands::Stats { path, format } => {
            let exit_code = stats::run(&path, &format);
            std::process::exit(exit_code);
        }
    }
}
