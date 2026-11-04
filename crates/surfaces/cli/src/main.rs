//! The `specforge` binary: each command's arguments, the terminal it writes to and asks from, and the exit code
//! it ends with. A core command turns its arguments into a request to `specforge-ops` and presents the outcome
//! (ADR 0029); `specforge mcp` serves `specforge-mcp` over stdio. Extension commands are read from the loaded
//! extensions' declarations.

#![deny(clippy::print_stderr)]

mod add;
mod analyze;
mod asking;
#[cfg(test)]
#[path = "../build/commit.rs"]
mod build_commit;
mod caller;
mod check;
mod collect;
mod doctor;
mod edit;
mod effects;
mod explain;
mod explore;
mod export;
mod extension_authoring;
mod extension_command;
mod extensions;
mod format;
mod grants;
mod harness;
mod hook;
#[cfg(test)]
mod hosts_tests;
mod infer_guide;
mod infer_status;
mod init;
mod install;
mod login;
mod mcp;
mod migrate;
mod model;
mod new;
mod notes;
mod options;
mod outcome;
mod outline;
mod pick;
mod project;
mod prompt;
mod providers;
mod publish;
mod query;
mod remove;
mod review;
mod scaffold;
mod search;
mod settings;
#[cfg(test)]
mod settings_tests;
mod stats;
mod terminal;
mod trace;
mod update;
mod verbs;
mod watch;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use outcome::Exit;
use specforge_effects::Ports;
use std::sync::Arc;
use terminal::{Stream, Terminal};

/// What `--version` prints after `specforge `: the package version, then the commit the build was made from
/// (when it knew it) and the extension protocol this binary speaks, `0.3.0 (9229143f5, protocol 2.0.0)`.
fn version() -> String {
    let protocol = specforge_protocol_types::PROTOCOL_VERSION;
    match env!("SPECFORGE_COMMIT") {
        "" => format!("{} (protocol {protocol})", env!("CARGO_PKG_VERSION")),
        commit => format!(
            "{} ({commit}, protocol {protocol})",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

#[derive(Parser)]
#[command(
    name = "specforge",
    version = version(),
    about = "SpecForge compiler",
    after_help = "Extensions add commands: specforge <extension> <command>, e.g. `specforge product features`.\n`specforge <extension> --help` lists an extension's commands."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new SpecForge project
    Init(init::Args),
    /// Validate .spec files and report diagnostics
    Check(check::Args),
    /// Export spec graph to stdout in various formats
    Export(export::Args),
    /// Output the Graph Protocol schema
    Schema(export::SchemaArgs),
    /// Render the logical data model (entity kinds, fields, relationships)
    Model(model::Args),
    /// Render the extension architecture (dependencies, enhancements, contributions)
    Outline(outline::Args),
    /// Query the graph at multiple resolutions
    Query(query::Args),
    /// Show an entity's thread along its kind's route, its traceability chain, or every entity's
    Trace(trace::Args),
    /// Format .spec files
    Format(format::Args),
    /// Show project statistics
    Stats(stats::Args),
    /// Where to start reading the graph: starting points, hubs and unconnected entities
    Explore(explore::Args),
    /// Coverage gaps around an entity, or of the whole project
    Review(review::Args),
    /// What to look for in code to write a kind's entities
    InferGuide(infer_guide::Args),
    /// Render an extension's prompt: `specforge prompt <extension>.<prompt> [arguments]`
    Prompt(prompt::Args),
    /// Install an extension
    Add(add::Args),
    /// Remove an installed extension
    Remove(remove::Args),
    /// List the project's extensions: enabled, locked or loaded
    Extensions(extensions::Args),
    /// Publish an extension to the registry that serves its name: its binary,
    /// and the declaration read from it as the package's manifest
    Publish(publish::Args),
    /// Search for extensions in registries
    Search(search::Args),
    /// Run static analysis passes over the compiled project
    Analyze(analyze::Args),
    /// Watch a project and rebuild incrementally on changes
    Watch(watch::Args),
    New(new::Args),
    /// Set a field (or --title) of an entity
    Set(edit::SetArgs),
    /// Remove a field of an entity
    Unset(edit::UnsetArgs),
    /// Add a reference (or --verify obligation) to an entity
    Link(edit::LinkArgs),
    /// Remove a reference (or --verify obligation) from an entity
    Unlink(edit::LinkArgs),
    /// Delete an entity
    Delete(edit::DeleteArgs),
    /// Update installed extensions to latest compatible versions
    Update(update::Args),
    /// Authenticate with a registry
    Login(login::Args),
    /// Remove registry credentials
    Logout(login::LogoutArgs),
    /// List configured providers
    Providers(providers::Args),
    /// List each extension's settings: the value, where it came from, what it gates and each secret's status
    ///
    /// `settings set` and `settings unset` write one setting of specforge.json (ADR 0296).
    Settings(settings::Args),
    /// List what the project's extensions may make the host do and this machine's approvals of it
    ///
    /// `grants approve` and `grants revoke` change the approvals, kept in the user's consent store (ADR 0335).
    Grants(grants::Args),
    /// Run the project's test runners and record which entities their tests prove
    Collect(collect::Args),
    /// The harness ledger: specforge-harness records the values only a command's plan sets
    Harness {
        #[command(subcommand)]
        action: harness::Action,
    },
    /// Run one hook for a host's event: read from stdin, answered on stdout in the host's own protocol
    Hook(hook::Args),
    /// Render the project's agents, skills, slash commands and hooks into a host's files, idempotently
    Install(install::InstallArgs),
    /// Remove exactly the host files and entries install wrote
    Uninstall(install::UninstallArgs),
    /// Run health checks on installed extensions
    Doctor(doctor::Args),
    /// Start MCP server (JSON-RPC over stdio)
    Mcp(mcp::Args),
    /// Migrate .spec files to the current format: the version header, a title for every block that has none, and the reference lists a file's blocks repeat moved into its `defaults`
    Migrate(migrate::Args),
    /// Generate shell completions: the built-in commands, and the commands
    /// of the extensions the project at --path enables
    Completions {
        /// Target shell
        #[arg(value_enum)]
        shell: Shell,

        #[command(flatten)]
        project: project::ProjectArg,
    },
    /// Explain a diagnostic code
    Explain(explain::Args),
    /// Show inference progress (analyzed files, gaps, stale entries)
    InferStatus(infer_status::Args),
    /// Extension authoring commands
    Extension {
        #[command(subcommand)]
        action: extension_authoring::Action,
    },
    /// Stat, list, search, change and digest files in the project's roots: the Fs port for people and agents
    /// (ADR 0339)
    Files {
        #[command(subcommand)]
        action: verbs::files::Action,
    },
    /// Send an HTTP request under the project's effects ceiling: its answer as JSON, a declared secret attached by
    /// --secret and never shown (ADR 0339)
    Http {
        #[command(subcommand)]
        action: verbs::http::Action,
    },
    /// Read the repository the project is in, with no git program: its place, HEAD, revisions, status and
    /// configuration, as JSON (ADR 0339)
    Git {
        #[command(subcommand)]
        action: verbs::git::Action,
    },
    /// Run a program: its output and exit pass through (124 at --deadline, 127 not found, 126 cannot start), or
    /// --format json captures them (ADR 0339)
    Run(verbs::run::RunArgs),
    /// An extension's command: `specforge <extension> <command>`, e.g.
    /// `specforge product features` (the project's extensions declare them)
    #[command(external_subcommand)]
    External(Vec<String>),
}

/// The command tree `specforge` declares, with `new`'s command line ([`new::command`]) in place of the words its
/// clap tree takes unread: what completions and the tree's tests read.
fn declared() -> clap::Command {
    Cli::command().mut_subcommand("new", |tree| {
        new::command(None).display_order(tree.get_display_order())
    })
}

fn main() {
    // A program the runner started is in its own group, out of the terminal's reach: Ctrl-C ends it here.
    specforge_process::stop_on_interrupt();
    let cli = Cli::try_parse().unwrap_or_else(|error| {
        // clap exits 2 on a usage error, which a host reads as a block; `hook` refuses with 1 (ADR 0324).
        let hook = std::env::args().nth(1).as_deref() == Some("hook");
        if hook && error.use_stderr() {
            let _ = error.print();
            std::process::exit(1);
        }
        error.exit()
    });
    let terminal = Terminal::process();
    let ports = ports();

    let exit_code = match cli.command {
        // An extension command's exit code is its own (ADR 0011).
        Commands::External(argv) => {
            let builtins: Vec<String> = Cli::command()
                .get_subcommands()
                .map(|c| c.get_name().to_string())
                .collect();
            extension_command::run(&argv, &builtins, &terminal, &ports)
        }
        // A program's exit code is its own (ADR 0339 D4).
        Commands::Run(args) => args.run(&terminal, &ports),
        command => run(command, &terminal, &ports).code(),
    };
    terminal.flush();
    std::process::exit(exit_code);
}

/// The CLI's composition root (ADR 0335): the effect plane's adapters, built once and passed to every command.
fn ports() -> Ports {
    let process: specforge_effects::SharedProcess = Arc::new(specforge_process::Runner);
    Ports {
        git: Arc::new(specforge_git::Gix),
        process,
        http: Arc::new(specforge_http::Client::new()),
        fs: Arc::new(specforge_fs::LocalFs),
    }
}

/// `specforge completions`: the built-ins, and the commands the extensions
/// of the project at `--path` contribute (`specforge product ...`). A path
/// that does not exist is refused like on every command.
fn completions(shell: Shell, project: &project::ProjectArg, terminal: &Terminal) -> Exit {
    let root = match project.root() {
        Ok(root) => root,
        Err(error) => {
            return outcome::Refusal::fixed().report(terminal, &error);
        }
    };
    let mut cmd = extension_command::with_extension_commands(declared(), &root);
    clap_complete::generate(
        shell,
        &mut cmd,
        "specforge",
        &mut terminal.writer(Stream::Out),
    );
    Exit::Passed
}

/// Run one core command at `terminal`.
fn run(command: Commands, terminal: &Terminal, ports: &Ports) -> Exit {
    match command {
        Commands::Init(args) => args.run(terminal),
        Commands::Check(args) => args.run(terminal, ports),
        Commands::Export(args) => args.run(terminal, ports),
        Commands::Schema(args) => args.run(terminal, ports),
        Commands::Model(args) => args.run(terminal, ports),
        Commands::Outline(args) => args.run(terminal, ports),
        Commands::Query(args) => args.run(terminal, ports),
        Commands::Trace(args) => args.run(terminal, ports),
        Commands::Format(args) => args.run(terminal),
        Commands::Stats(args) => args.run(terminal, ports),
        Commands::Explore(args) => args.run(terminal, ports),
        Commands::Review(args) => args.run(terminal, ports),
        Commands::InferGuide(args) => args.run(terminal, ports),
        Commands::Prompt(args) => args.run(terminal, ports),
        Commands::Add(args) => args.run(terminal, ports),
        Commands::Analyze(args) => args.run(terminal, ports),
        Commands::Watch(args) => args.run(terminal, ports),
        Commands::New(args) => args.run(terminal, ports),
        Commands::Set(args) => args.run(terminal, ports),
        Commands::Unset(args) => args.run(terminal, ports),
        Commands::Link(args) => args.run(true, terminal, ports),
        Commands::Unlink(args) => args.run(false, terminal, ports),
        Commands::Delete(args) => args.run(terminal, ports),
        Commands::Remove(args) => args.run(terminal, ports),
        Commands::Extensions(args) => args.run(terminal, ports),
        Commands::Publish(args) => args.run(terminal, ports),
        Commands::Search(args) => args.run(terminal, ports),
        Commands::Update(args) => args.run(terminal, ports),
        Commands::Login(args) => args.run(terminal, ports),
        Commands::Logout(args) => args.run(terminal, ports),
        Commands::Providers(args) => args.run(terminal, ports),
        Commands::Settings(args) => args.run(terminal, ports),
        Commands::Grants(args) => args.run(terminal, ports),
        Commands::Collect(args) => args.run(terminal, ports),
        Commands::Harness { action } => action.run(terminal, ports),
        Commands::Hook(args) => args.run(terminal, ports),
        Commands::Install(args) => args.run(terminal, ports),
        Commands::Uninstall(args) => args.run(terminal, ports),
        Commands::Doctor(args) => args.run(terminal, ports),
        Commands::Mcp(args) => args.run(terminal, ports),
        Commands::Completions { shell, project } => completions(shell, &project, terminal),
        Commands::Explain(args) => args.run(terminal),
        Commands::Migrate(args) => args.run(terminal),
        Commands::InferStatus(args) => args.run(terminal, ports),
        Commands::Files { action } => action.run(terminal, ports),
        Commands::Http { action } => action.run(terminal, ports),
        Commands::Git { action } => action.run(terminal, ports),
        Commands::External(_) => unreachable!("main runs an extension command itself"),
        Commands::Run(_) => unreachable!("main runs a program itself"),
        Commands::Extension { action } => action.run(terminal, ports),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plain long help of `command` and of each subcommand under it,
    /// by the snapshot name `help_root` or `help_<path joined by _>`.
    fn pin_help(command: &clap::Command, path: &[String]) {
        let name = if path.is_empty() {
            "help_root".to_string()
        } else {
            format!("help_{}", path.join("_"))
        };
        insta::assert_snapshot!(name, command.clone().render_long_help().to_string());
        for sub in command.get_subcommands() {
            // The generated `help` command and the external-subcommand
            // placeholder have no help of their own to pin.
            if sub.get_name() == "help" || sub.is_allow_external_subcommands_set() {
                continue;
            }
            let mut below = path.to_vec();
            below.push(sub.get_name().to_string());
            pin_help(sub, &below);
        }
    }

    /// Every core command's `--help` is pinned, so moving the arguments out
    /// of this file cannot change a byte of it (plan 11).
    #[test]
    fn every_core_command_help_is_pinned() {
        let mut root = declared();
        root.build();
        pin_help(&root, &[]);
    }

    /// `CORE_COMMANDS`, which the registry reads to report E114, is the clap tree's list of core commands.
    #[test]
    fn core_commands_are_the_declared_tree() {
        let mut root = declared();
        root.build();
        let mut tree: Vec<&str> = root
            .get_subcommands()
            .filter(|sub| sub.get_name() != "help" && !sub.is_allow_external_subcommands_set())
            .map(clap::Command::get_name)
            .collect();
        tree.sort_unstable();
        let mut listed = specforge_common::structural::CORE_COMMANDS.to_vec();
        listed.sort_unstable();
        assert_eq!(tree, listed);
    }

    /// The commands that read no project: they take no `--path`.
    const NO_PROJECT: [&str; 7] = [
        "init",
        // The host runs it from its own cwd, which names the project.
        "hook",
        "mcp",
        "extension",
        "extension init",
        "extension build",
        "extension validate",
    ];

    /// Every command of the tree under `command` (not the generated `help`
    /// nor the external-subcommand placeholder), by its path from the root.
    fn commands<'a>(
        command: &'a clap::Command,
        at: &str,
        found: &mut Vec<(String, &'a clap::Command)>,
    ) {
        for sub in command.get_subcommands() {
            if sub.get_name() == "help" || sub.is_allow_external_subcommands_set() {
                continue;
            }
            let name = if at.is_empty() {
                sub.get_name().to_string()
            } else {
                format!("{at} {}", sub.get_name())
            };
            found.push((name.clone(), sub));
            commands(sub, &name, found);
        }
    }

    #[specforge_test_macros::test(
        behavior = "locate_project_root",
        verify = "every command that reads a project takes the one --path option, and none takes a project path as a positional argument"
    )]
    fn every_command_that_reads_a_project_takes_the_project_arg() {
        let mut root = declared();
        root.build();
        let mut found = Vec::new();
        commands(&root, "", &mut found);
        for name in NO_PROJECT {
            assert!(
                found.iter().any(|(command, _)| command == name),
                "NO_PROJECT names `{name}`, which is no command"
            );
        }
        let project = crate::project::ProjectArg::arg();
        let shown = |value: Option<&clap::builder::StyledStr>| value.map(ToString::to_string);
        for (name, command) in &found {
            let paths: Vec<&clap::Arg> = command
                .get_arguments()
                .filter(|argument| {
                    argument.get_id() == crate::project::ProjectArg::NAME
                        || argument.get_long() == Some(crate::project::ProjectArg::NAME)
                })
                .collect();
            if NO_PROJECT.contains(&name.as_str()) {
                assert!(
                    paths.is_empty(),
                    "`specforge {name}` reads no project but takes --path"
                );
                continue;
            }
            // A command with subcommands only dispatches; its leaves read. `settings` and `grants` also list, and
            // take --path for the listing (their subcommands conflict with the listing's flags).
            if command.has_subcommands() && name != "settings" && name != "grants" {
                assert!(
                    paths.is_empty(),
                    "`specforge {name}` dispatches but takes --path"
                );
                continue;
            }
            let [path] = paths.as_slice() else {
                panic!(
                    "`specforge {name}` reads a project and takes no --path (flatten ProjectArg, or list it in NO_PROJECT)"
                );
            };
            assert!(
                !path.is_positional(),
                "`specforge {name}` takes its project positionally"
            );
            assert_eq!(path.get_long(), project.get_long(), "`specforge {name}`");
            assert_eq!(
                shown(path.get_help()),
                shown(project.get_help()),
                "`specforge {name}`"
            );
            assert_eq!(
                path.get_default_values(),
                project.get_default_values(),
                "`specforge {name}`"
            );
        }
    }

    /// Every argument of `command` and of each subcommand under it.
    fn all_arguments(command: &clap::Command, found: &mut Vec<(String, clap::Arg)>) {
        for argument in command.get_arguments() {
            found.push((command.get_name().to_string(), argument.clone()));
        }
        for sub in command.get_subcommands() {
            all_arguments(sub, found);
        }
    }

    #[specforge_test_macros::test(
        behavior = "report_command_outcome",
        verify = "every core command with a JSON form takes --format human|json, and none takes --json"
    )]
    fn every_json_output_is_asked_for_with_format() {
        let mut root = declared();
        root.build();
        let mut arguments = Vec::new();
        all_arguments(&root, &mut arguments);
        for (command, argument) in &arguments {
            assert_ne!(
                argument.get_long(),
                Some("json"),
                "`specforge {command}` takes --json"
            );
        }
        for name in ["analyze", "watch", "format", "new"] {
            let command = root.find_subcommand(name).expect("a core command");
            let format = command
                .get_arguments()
                .find(|argument| argument.get_id() == "format")
                .unwrap_or_else(|| panic!("`specforge {name}` takes --format"));
            let values: Vec<String> = format
                .get_possible_values()
                .iter()
                .map(|value| value.get_name().to_string())
                .collect();
            assert_eq!(values, ["human", "json"], "`specforge {name} --format`");
            let defaults: Vec<_> = format.get_default_values().iter().collect();
            assert_eq!(defaults, ["human"], "`specforge {name} --format` default");
        }
    }

    /// `specforge query --depth` defaults to the query operation's constant,
    /// the one `specforge.query`'s schema advertises (plan 08 T8).
    #[test]
    fn the_query_depth_default_is_the_operations() {
        let command = Cli::command();
        let query = command.find_subcommand("query").expect("a query command");
        let depth = query
            .get_arguments()
            .find(|argument| argument.get_id() == "depth")
            .expect("query takes --depth");
        let defaults: Vec<String> = depth
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            defaults,
            [specforge_ops::read::query::DEFAULT_DEPTH.to_string()]
        );
    }
}
