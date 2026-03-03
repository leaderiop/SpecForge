pub mod add;
pub mod check;
pub mod gen_cmd;
pub mod graph_cmd;
pub mod init;
pub mod migrate;
pub mod render;
pub mod stats;
pub mod trace;
pub mod verify_cmd;
pub mod watch;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "specforge", version, about = "SpecForge — a specification compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a plugin to the current project
    Add(add::AddArgs),
    /// Check spec files for errors and warnings
    Check(check::CheckArgs),
    /// Generate code from spec entities (types, ports, test stubs)
    Gen(gen_cmd::GenArgs),
    /// Initialize a new specforge project
    Init(init::InitArgs),
    /// Migrate spec files to the latest format version
    Migrate(migrate::MigrateArgs),
    /// Render spec graph to a file format
    Render(render::RenderArgs),
    /// Print spec graph in DOT format to stdout
    Graph(graph_cmd::GraphArgs),
    /// Print project statistics to stdout
    Stats(stats::StatsArgs),
    /// Trace entity connections and report traceability gaps
    Trace(trace::TraceArgs),
    /// Verify generated files against spec (checksum + adapter checks)
    Verify(verify_cmd::VerifyArgs),
    /// Watch spec files and recompile on changes
    Watch(watch::WatchArgs),
}
