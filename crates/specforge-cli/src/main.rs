mod commands;
mod formatter;
mod pipeline;

use clap::Parser;
use commands::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::Add(args) => commands::add::run(args),
        Command::Cache(args) => commands::cache::run(args),
        Command::Check(args) => commands::check::run(args),
        Command::Doctor(args) => commands::doctor::run(args),
        Command::Coverage(args) => commands::coverage::run(args),
        Command::Gen(args) => commands::gen_cmd::run(args),
        Command::Init(args) => commands::init::run(args),
        Command::Migrate(args) => commands::migrate::run(args),
        Command::Package(args) => commands::package::run(args),
        Command::Plugin(args) => commands::plugin::run(args),
        Command::Plugins(args) => commands::plugins::run(args),
        Command::Providers(args) => commands::providers::run(args),
        Command::Remove(args) => commands::remove::run(args),
        Command::Render(args) => commands::render::run(args),
        Command::Schema(args) => commands::schema::run(args),
        Command::Graph(args) => commands::graph_cmd::run(args),
        Command::Stats(args) => commands::stats::run(args),
        Command::Trace(args) => commands::trace::run(args),
        Command::Verify(args) => commands::verify_cmd::run(args),
        Command::Watch(args) => commands::watch::run(args),
    };

    std::process::exit(exit_code);
}
