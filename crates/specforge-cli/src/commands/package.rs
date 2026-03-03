use clap::{Args, Subcommand};
use std::path::PathBuf;

pub mod package_build;
pub mod package_init;
pub mod package_publish;
pub mod package_test;

#[derive(Args)]
pub struct PackageArgs {
    #[command(subcommand)]
    pub command: PackageCommand,
}

#[derive(Subcommand)]
pub enum PackageCommand {
    /// Scaffold a new Wasm package project
    Init {
        /// Package name (e.g., "my-package")
        name: String,
        /// Directory to create the project in (default: ./<name>)
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Build the package to a .wasm binary
    Build {
        /// Path to package project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Test the package locally against fixture spec files
    Test {
        /// Path to package project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Package for distribution
    Publish {
        /// Path to package project (default: current directory)
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Output directory for the package (default: ./dist)
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

pub fn run(args: PackageArgs) -> i32 {
    match args.command {
        PackageCommand::Init { name, dir } => package_init::run(&name, dir),
        PackageCommand::Build { path } => package_build::run(&path),
        PackageCommand::Test { path } => package_test::run(&path),
        PackageCommand::Publish { path, out } => package_publish::run(&path, out),
    }
}
