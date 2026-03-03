use clap::{Args, Subcommand};
use std::path::PathBuf;

pub mod plugin_build;
pub mod plugin_init;
pub mod plugin_publish;
pub mod plugin_test;

#[derive(Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// Scaffold a new Wasm plugin project
    Init {
        /// Plugin name (e.g., "my-plugin")
        name: String,
        /// Directory to create the project in (default: ./<name>)
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Build the plugin to a .wasm binary
    Build {
        /// Path to plugin project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Test the plugin locally against fixture spec files
    Test {
        /// Path to plugin project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Package the plugin for distribution
    Publish {
        /// Path to plugin project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Output directory for the package (default: ./dist)
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

pub fn run(args: PluginArgs) -> i32 {
    match args.command {
        PluginCommand::Init { name, dir } => plugin_init::run(&name, dir),
        PluginCommand::Build { path } => plugin_build::run(&path),
        PluginCommand::Test { path } => plugin_test::run(&path),
        PluginCommand::Publish { path, out } => plugin_publish::run(&path, out),
    }
}
