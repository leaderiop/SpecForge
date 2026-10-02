//! `specforge <extension> <command> [args]`: the commands extensions
//! contribute (ADR 0008).
//!
//! Any first argument that is not a built-in command names an extension by
//! its short name (`product` for `@specforge/product`); `ext:command` is the
//! same as `ext command`. The project is the one at `--path` (default `.`),
//! an option every extension command has. Its extensions declare the
//! commands: this module builds their command line from the declared args
//! (a required arg is positional, in declaration order; any other is a
//! `--flag`), compiles the project, and runs the command's `cmd__` export
//! over the graph (`specforge_ops::command`). The export's stdout and stderr
//! are printed as they are, and its exit code is the CLI's.

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};
use specforge_ops::command::{ExtensionCommand, ext_short, extension_commands, run_command};
use specforge_registry::{CommandArg, CommandArgType};
use std::io::Write;
use std::path::PathBuf;

/// The host's own option on every extension command: where the project is.
const PATH: &str = "path";

/// Run the extension command `argv` names (`argv[0]` is the extension).
/// `builtins` are the CLI's own commands, suggested for a name no extension
/// has.
pub fn run(argv: &[String], builtins: &[String]) -> i32 {
    let Some((first, rest)) = argv.split_first() else {
        return 2;
    };
    // `product:features` is `product features`.
    let (ext, mut rest) = match first.split_once(':') {
        Some((ext, command)) => {
            let mut rest = rest.to_vec();
            rest.insert(0, command.to_string());
            (ext.to_string(), rest)
        }
        None => (first.clone(), rest.to_vec()),
    };

    let root = project_path(&rest);
    let runtime = specforge_component::project_runtime(&root);
    let project = specforge_project::CompiledProject::compile(&root, Some(&runtime));
    let build = &project.env.registries;
    let commands: Vec<ExtensionCommand> = extension_commands(build)
        .into_iter()
        .filter(|c| ext_short(&build.manifests, c.extension) == ext)
        .collect();
    if commands.is_empty() {
        eprintln!(
            "error: unrecognized subcommand '{ext}': no built-in command, and no extension of the project at {} with that name contributes commands",
            root.display()
        );
        if let Some(close) =
            specforge_common::find_close_match(&ext, builtins.iter().map(String::as_str))
        {
            eprintln!("\n  tip: a similar subcommand exists: '{close}'");
        }
        eprintln!("\nFor more information, try 'specforge --help'.");
        return 2;
    }

    rest.insert(0, format!("specforge {ext}"));
    let matches = match command_line(&ext, &commands).try_get_matches_from(rest) {
        Ok(matches) => matches,
        Err(e) => {
            let _ = e.print();
            return e.exit_code();
        }
    };
    let Some((name, matches)) = matches.subcommand() else {
        return 2;
    };
    let Some(command) = commands.iter().find(|c| c.cli_name() == name) else {
        return 2;
    };

    let args = arg_values(&command.contribution.args, matches);
    let cwd = std::fs::canonicalize(&root).unwrap_or(root);
    match run_command(
        &runtime,
        command.extension,
        &command.contribution.export,
        &project.graph,
        &args,
        &cwd,
    ) {
        Ok(output) => {
            let _ = std::io::stdout().write_all(&output.stdout);
            let _ = std::io::stderr().write_all(&output.stderr);
            output.exit_code
        }
        Err(diagnostic) => {
            eprintln!("{}", crate::export::render_plain(&diagnostic));
            1
        }
    }
}

/// `--path <dir>` or `--path=<dir>` among `args`, else `.`: the project the
/// extensions are loaded from, needed before the command line can be parsed.
fn project_path(args: &[String]) -> PathBuf {
    let flag = format!("--{PATH}");
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if *arg == flag {
            if let Some(value) = args.next() {
                return PathBuf::from(value);
            }
        } else if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
            return PathBuf::from(value);
        }
    }
    PathBuf::from(".")
}

/// `specforge <ext>`'s command line: one subcommand per command.
fn command_line(ext: &str, commands: &[ExtensionCommand]) -> Command {
    let mut cli = Command::new(ext.to_string())
        .bin_name(format!("specforge {ext}"))
        .subcommand_required(true)
        .arg_required_else_help(true);
    for command in commands {
        let contribution = command.contribution;
        let mut sub = Command::new(command.cli_name()).about(contribution.title.clone());
        if !contribution.description.is_empty() {
            sub = sub.long_about(contribution.description.clone());
        }
        for arg in &contribution.args {
            sub = sub.arg(declared_arg(arg));
        }
        if !contribution.args.iter().any(|a| a.name == PATH) {
            sub = sub.arg(
                Arg::new(PATH)
                    .long(PATH)
                    .value_name("PATH")
                    .default_value(".")
                    .help("Path to the project"),
            );
        }
        cli = cli.subcommand(sub);
    }
    cli
}

/// A declared arg on the command line: required ones are positional.
fn declared_arg(declared: &CommandArg) -> Arg {
    let mut arg = Arg::new(declared.name.clone());
    if !matches!(declared.arg_type, CommandArgType::Bool) {
        arg = arg.value_name(declared.name.to_uppercase());
    }
    if declared.required {
        arg = arg.required(true);
    } else {
        arg = arg.long(declared.name.replace('_', "-"));
    }
    if let Some(help) = &declared.description {
        arg = arg.help(help.clone());
    }
    arg = match &declared.arg_type {
        CommandArgType::Bool => arg.action(ArgAction::SetTrue),
        CommandArgType::Integer => arg.value_parser(clap::value_parser!(i64)),
        CommandArgType::Enum { values } => arg.value_parser(PossibleValuesParser::new(values)),
        CommandArgType::String | CommandArgType::Path => arg,
    };
    if let Some(default) = &declared.default_value
        && !matches!(declared.arg_type, CommandArgType::Bool)
    {
        arg = arg.default_value(default.clone());
    }
    arg
}

/// The args the command line set (or defaulted), typed as declared.
fn arg_values(declared: &[CommandArg], matches: &ArgMatches) -> Map<String, Value> {
    let mut args = Map::new();
    for arg in declared {
        let value = match arg.arg_type {
            CommandArgType::Bool => Some(Value::Bool(matches.get_flag(&arg.name))),
            CommandArgType::Integer => matches.get_one::<i64>(&arg.name).map(|n| Value::from(*n)),
            _ => matches
                .get_one::<String>(&arg.name)
                .map(|s| Value::from(s.as_str())),
        };
        if let Some(value) = value {
            args.insert(arg.name.clone(), value);
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::CommandContribution;

    fn arg(
        name: &str,
        arg_type: CommandArgType,
        required: bool,
        default: Option<&str>,
    ) -> CommandArg {
        CommandArg {
            name: name.into(),
            arg_type,
            required,
            default_value: default.map(str::to_string),
            description: None,
        }
    }

    fn contribution() -> CommandContribution {
        CommandContribution {
            id: "milestone_completion".into(),
            title: "Show progress".into(),
            description: String::new(),
            category: None,
            export: "cmd__x_milestone_completion".into(),
            args: vec![
                arg("milestone", CommandArgType::String, true, None),
                arg("limit", CommandArgType::Integer, false, None),
                arg("all_kinds", CommandArgType::Bool, false, None),
                arg(
                    "format",
                    CommandArgType::Enum {
                        values: vec!["human".into(), "json".into()],
                    },
                    false,
                    Some("human"),
                ),
            ],
            sandbox: None,
        }
    }

    fn parse(argv: &[&str]) -> Result<Map<String, Value>, clap::Error> {
        let c = contribution();
        let commands = [ExtensionCommand {
            extension: "@acme/x",
            contribution: &c,
        }];
        let matches = command_line("x", &commands)
            .try_get_matches_from(std::iter::once("specforge x").chain(argv.iter().copied()))?;
        let (name, sub) = matches.subcommand().unwrap();
        assert_eq!(name, "milestone-completion");
        assert_eq!(sub.get_one::<String>(PATH).map(String::as_str), Some("."));
        Ok(arg_values(&c.args, sub))
    }

    #[test]
    fn declared_args_become_typed_values() {
        let args = parse(&["milestone-completion", "m1", "--limit", "3", "--all-kinds"]).unwrap();
        assert_eq!(
            Value::Object(args),
            serde_json::json!({"milestone": "m1", "limit": 3, "all_kinds": true, "format": "human"})
        );
    }

    #[test]
    fn the_command_line_refuses_what_the_declaration_does() {
        // A required arg is positional and required; an enum takes its values.
        assert!(parse(&["milestone-completion"]).is_err());
        assert!(parse(&["milestone-completion", "m1", "--format", "xml"]).is_err());
        assert!(parse(&["milestone-completion", "m1", "--limit", "many"]).is_err());
    }

    #[test]
    fn the_project_path_is_found_before_parsing() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(project_path(&args(&["features"])), PathBuf::from("."));
        assert_eq!(
            project_path(&args(&["features", "--path", "/p", "--limit", "1"])),
            PathBuf::from("/p")
        );
        assert_eq!(
            project_path(&args(&["--path=/q", "features"])),
            PathBuf::from("/q")
        );
    }
}
