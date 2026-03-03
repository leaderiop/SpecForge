pub mod add;
pub mod cache;
pub mod check;
pub mod coverage;
pub mod doctor;
pub mod gen_cmd;
pub mod graph_cmd;
pub mod init;
pub mod migrate;
pub mod package;
pub mod plugin;
pub mod plugins;
pub mod providers;
pub mod remove;
pub mod render;
pub mod schema;
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
    /// Manage AOT compilation cache
    Cache(cache::CacheArgs),
    /// Check spec files for errors and warnings
    Check(check::CheckArgs),
    /// Diagnose plugin enhancements, conflicts, and configuration
    Doctor(doctor::DoctorArgs),
    /// Compute test coverage from specforge-report.json files
    Coverage(coverage::CoverageArgs),
    /// Generate code from spec entities (types, ports, test stubs)
    Gen(gen_cmd::GenArgs),
    /// Initialize a new specforge project
    Init(init::InitArgs),
    /// Migrate spec files to the latest format version
    Migrate(migrate::MigrateArgs),
    /// Wasm package authoring commands (init, build, test, publish)
    Package(package::PackageArgs),
    /// Wasm plugin authoring commands (init, build, test, publish) [deprecated: use `package`]
    Plugin(plugin::PluginArgs),
    /// List installed plugins
    Plugins(plugins::PluginsArgs),
    /// List configured providers
    Providers(providers::ProvidersArgs),
    /// Remove a plugin from the current project
    Remove(remove::RemoveArgs),
    /// Render spec graph to a file format
    Render(render::RenderArgs),
    /// Print spec graph in DOT format to stdout
    Graph(graph_cmd::GraphArgs),
    /// Print the JSON Schema for specforge.json
    Schema(schema::SchemaArgs),
    /// Print project statistics to stdout
    Stats(stats::StatsArgs),
    /// Trace entity connections and report traceability gaps
    Trace(trace::TraceArgs),
    /// Verify generated files against spec (checksum + adapter checks)
    Verify(verify_cmd::VerifyArgs),
    /// Watch spec files and recompile on changes
    Watch(watch::WatchArgs),
}
