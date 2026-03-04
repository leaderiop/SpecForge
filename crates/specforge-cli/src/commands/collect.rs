pub mod collect_rust;

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CollectArgs {
    #[command(subcommand)]
    pub command: CollectCommand,
}

#[derive(Subcommand)]
pub enum CollectCommand {
    /// Collect Rust test results from JUnit XML or libtest JSON
    Rust(collect_rust::CollectRustArgs),
}

pub fn run(args: CollectArgs) -> i32 {
    match args.command {
        CollectCommand::Rust(rust_args) => collect_rust::run(rust_args),
    }
}
