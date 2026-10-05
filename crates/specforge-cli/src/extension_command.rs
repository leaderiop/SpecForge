//! `specforge <extension> <command> [args]`: the commands extensions
//! contribute (ADR 0008).
//!
//! Any first argument that is not a built-in command names an extension by
//! its short name (`product` for `@specforge/product`); `ext:command` is the
//! same as `ext command`. The project is the one at `--path` (default `.`),
//! and the output the one `--format` asks for (`human`, the default, or
//! `json`): options every extension command has, the host's, which no
//! declared arg may take (ADR 0011). Its extensions declare the
//! commands: this module builds their command line from the declared args
//! (a required arg is positional, in declaration order; any other is a
//! `--flag`), compiles the project, and runs the command's `cmd__` export
//! over the graph with the format and today's date (UTC)
//! (`specforge_ops::command`). The export's stdout and stderr are printed as
//! they are, and its exit code is the CLI's.

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Map, Value};
use specforge_ops::command::{
    CommandContext, CommandFormat, ExtensionCommand, ext_short, extension_commands, refusal,
    run_command,
};
use specforge_project::Environment;
use specforge_registry::{CommandArg, CommandArgType};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The host's own option on every extension command: where the project is.
const PATH: &str = "path";

/// The host's own option on every extension command: the output asked for.
const FORMAT: &str = "format";

/// Run the extension command `argv` names (`argv[0]` is the extension).
/// `builtins` are the CLI's own commands, suggested for a name no extension
/// has.
///
/// Only the project's environment (its config and its extensions' declared
/// surfaces) is loaded to route the command; the project's sources are read
/// and its graph built only once a declared command is matched.
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
    let env = Environment::load(&root, Some(&runtime));
    let all = extension_commands(&env);
    let commands: Vec<ExtensionCommand> = all
        .iter()
        .copied()
        .filter(|c| ext_short(&env.manifests, c.extension) == ext)
        .collect();
    if commands.is_empty() {
        eprintln!(
            "error: unrecognized subcommand '{ext}': no built-in command, and no extension of the project at {} with that name contributes commands",
            root.display()
        );
        let mut names: Vec<String> = builtins.to_vec();
        names.extend(all.iter().map(|c| ext_short(&env.manifests, c.extension)));
        if let Some(close) =
            specforge_common::find_close_match(&ext, names.iter().map(String::as_str))
        {
            eprintln!("\n  tip: a similar subcommand exists: '{close}'");
        }
        eprintln!("\nFor more information, try 'specforge --help'.");
        return 2;
    }
    if let Some(requested) = rest.first()
        && let Some(command) = commands.iter().find(|c| c.cli_name() == *requested)
        && let Some(why) = refusal(command.contribution)
    {
        eprintln!(
            "error: {}'s command '{requested}' cannot run on the command line: {why}",
            command.extension
        );
        return 2;
    }

    // Known before parsing, so a usage error is written in the format asked
    // for wherever `--format` is on the command line, or if parsing stops
    // before reaching it.
    let json = asks_for_json(&rest);
    rest.insert(0, format!("specforge {ext}"));
    let matches = match command_line(&ext, &commands).try_get_matches_from(rest) {
        Ok(matches) => matches,
        Err(e) if json && !is_display(&e) => {
            eprintln!("{}", usage_error(&e));
            return INVALID_INPUT_EXIT;
        }
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
    let context = CommandContext {
        format: format_value(matches),
        today: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    };
    let cwd = std::fs::canonicalize(&root).unwrap_or(root);
    match run_command(
        &runtime,
        command.extension,
        &command.contribution.export,
        &env.build_graph(),
        &args,
        &cwd,
        &context,
    ) {
        Ok(output) => {
            let _ = std::io::stdout().write_all(&output.stdout);
            let _ = std::io::stderr().write_all(&output.stderr);
            output.exit_code
        }
        Err(diagnostic) => {
            eprint!("{}", failed_run(&diagnostic, context.format));
            1
        }
    }
}

/// What the CLI writes to stderr when a command's export did not answer
/// (it trapped: E028), in the format asked for: under `json` one error
/// object of the shape commands write (`{code, message, suggestion?}`),
/// under `human` the diagnostic line.
fn failed_run(diagnostic: &specforge_common::Diagnostic, format: CommandFormat) -> String {
    match format {
        CommandFormat::Json => {
            let mut error = serde_json::json!({
                "code": diagnostic.code,
                "message": diagnostic.message,
            });
            if let Some(suggestion) = &diagnostic.suggestion {
                error["suggestion"] = Value::from(suggestion.as_str());
            }
            format!("{error}\n")
        }
        CommandFormat::Human => format!("{}\n", crate::export::render_plain(diagnostic)),
    }
}

/// `cli` with a subcommand per extension of the project at `root` that
/// contributes commands, as `specforge <ext>` routes them: what shell
/// completions are generated from. An extension whose short name is a
/// built-in command's is left out (the built-in wins), and so is any
/// command whose args [`refusal`] refuses.
pub fn with_extension_commands(mut cli: Command, root: &Path) -> Command {
    let runtime = specforge_component::project_runtime(root);
    let env = Environment::load(root, Some(&runtime));
    let mut by_ext: Vec<(String, Vec<ExtensionCommand>)> = Vec::new();
    for command in extension_commands(&env) {
        if refusal(command.contribution).is_some() {
            continue;
        }
        let short = ext_short(&env.manifests, command.extension);
        match by_ext.iter_mut().find(|(ext, _)| *ext == short) {
            Some((_, commands)) => commands.push(command),
            None => by_ext.push((short, vec![command])),
        }
    }
    for (ext, commands) in by_ext {
        if cli.find_subcommand(&ext).is_none() {
            let about = format!("{ext} extension commands");
            cli = cli.subcommand(command_line(&ext, &commands).about(about));
        }
    }
    cli
}

/// The exit code of a usage error clap catches, `INVALID_INPUT`'s: the
/// one commands give the usage errors they catch themselves (ADR 0011).
const INVALID_INPUT_EXIT: i32 = 2;

/// Whether `args` ask for `--format json` (`--format json` or
/// `--format=json`), read before the command line is parsed.
fn asks_for_json(args: &[String]) -> bool {
    let flag = format!("--{FORMAT}");
    let json = CommandFormat::Json.as_str();
    let mut args = args.iter().take_while(|a| *a != "--");
    while let Some(arg) = args.next() {
        if *arg == flag {
            if args.next().is_some_and(|v| v == json) {
                return true;
            }
        } else if arg.strip_prefix(&format!("{flag}=")) == Some(json) {
            return true;
        }
    }
    false
}

/// Whether `e` is clap's help or version rather than a usage error.
fn is_display(e: &clap::Error) -> bool {
    use clap::error::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// A usage error clap caught on an extension's command line, as the error
/// object commands write under `json` (`{code, message, suggestion?}`):
/// `INVALID_INPUT`, the message naming the arg as declared, as the SDK's
/// would (`status must be one of ..., got 'x'`, `missing required arg
/// 'milestone'`), and clap's suggestion when it has one.
fn usage_error(e: &clap::Error) -> Value {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    let text = |kind| match e.get(kind) {
        Some(ContextValue::String(s)) => Some(s.clone()),
        Some(ContextValue::Strings(s)) => s.first().cloned(),
        _ => None,
    };
    let arg = text(ContextKind::InvalidArg).map(|a| declared_name(&a));
    let value = text(ContextKind::InvalidValue).unwrap_or_default();
    let (message, suggestion) = match e.kind() {
        ErrorKind::InvalidValue => {
            let arg = arg.unwrap_or_default();
            let message = match e.get(ContextKind::ValidValue) {
                Some(ContextValue::Strings(values)) => {
                    format!("{arg} must be one of {}, got '{value}'", values.join(", "))
                }
                _ => format!("invalid value '{value}' for {arg}"),
            };
            (message, text(ContextKind::SuggestedValue))
        }
        ErrorKind::ValueValidation => {
            let arg = arg.unwrap_or_default();
            let integer = std::error::Error::source(e)
                .is_some_and(|s| s.downcast_ref::<std::num::ParseIntError>().is_some());
            let message = match std::error::Error::source(e) {
                _ if integer => format!("{arg} must be an integer, got '{value}'"),
                Some(why) => format!("invalid value '{value}' for {arg}: {why}"),
                None => format!("invalid value '{value}' for {arg}"),
            };
            (message, None)
        }
        ErrorKind::MissingRequiredArgument => {
            let names = match e.get(ContextKind::InvalidArg) {
                Some(ContextValue::Strings(args)) => args
                    .iter()
                    .map(|a| format!("'{}'", declared_name(a)))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            let s = if names.len() > 1 { "s" } else { "" };
            (
                format!("missing required arg{s} {}", names.join(", ")),
                None,
            )
        }
        ErrorKind::UnknownArgument => (
            format!(
                "unknown argument '{}'",
                text(ContextKind::InvalidArg).unwrap_or_default()
            ),
            text(ContextKind::SuggestedArg),
        ),
        ErrorKind::InvalidSubcommand => (
            format!(
                "unknown command '{}'",
                text(ContextKind::InvalidSubcommand).unwrap_or_default()
            ),
            text(ContextKind::SuggestedSubcommand),
        ),
        _ => {
            let rendered = e.render().to_string();
            let first = rendered.lines().next().unwrap_or_default();
            (
                first.strip_prefix("error: ").unwrap_or(first).to_string(),
                None,
            )
        }
    };
    let mut error = serde_json::json!({"code": "INVALID_INPUT", "message": message});
    if let Some(suggestion) = suggestion {
        error["suggestion"] = Value::from(suggestion);
    }
    error
}

/// The declared name of an arg as clap shows it: `<MILESTONE>` is
/// `milestone`, `--sort-order <SORT_ORDER>` is `sort_order`, `--details`
/// is `details` (a value name is the declared name upper-cased).
fn declared_name(shown: &str) -> String {
    match shown.split_once('<') {
        Some((_, rest)) => rest.split('>').next().unwrap_or_default().to_lowercase(),
        None => shown
            .trim_start_matches('-')
            .split(['=', ' '])
            .next()
            .unwrap_or_default()
            .replace('-', "_"),
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

/// `specforge <ext>`'s command line: one subcommand per command, none of
/// them refused by [`refusal`].
fn command_line(ext: &str, commands: &[ExtensionCommand]) -> Command {
    let mut cli = Command::new(ext.to_string())
        .bin_name(format!("specforge {ext}"))
        .subcommand_required(true)
        .arg_required_else_help(true);
    for command in commands {
        let contribution = command.contribution;
        if refusal(contribution).is_some() {
            continue;
        }
        let mut sub = Command::new(command.cli_name()).about(contribution.title.clone());
        if !contribution.description.is_empty() {
            sub = sub.long_about(contribution.description.clone());
        }
        for arg in &contribution.args {
            sub = sub.arg(declared_arg(arg));
        }
        sub = sub
            .arg(
                Arg::new(PATH)
                    .long(PATH)
                    .value_name("PATH")
                    .default_value(".")
                    .help("Path to the project"),
            )
            .arg(
                Arg::new(FORMAT)
                    .long(FORMAT)
                    .value_name("FORMAT")
                    .value_parser(PossibleValuesParser::new(
                        CommandFormat::ALL.map(CommandFormat::as_str),
                    ))
                    .default_value(CommandFormat::Human.as_str())
                    .help("Output format"),
            );
        cli = cli.subcommand(sub);
    }
    cli
}

/// A declared arg on the command line: required ones are positional, but a
/// bool, which is always a `--flag` (set or not).
fn declared_arg(declared: &CommandArg) -> Arg {
    let mut arg = Arg::new(declared.name.clone());
    let flag = matches!(declared.arg_type, CommandArgType::Bool);
    if !flag {
        arg = arg.value_name(declared.name.to_uppercase());
    }
    if declared.required && !flag {
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

/// The host's `--format` the command line set (or defaulted).
fn format_value(matches: &ArgMatches) -> CommandFormat {
    matches
        .get_one::<String>(FORMAT)
        .and_then(|f| CommandFormat::parse(f))
        .unwrap_or_default()
}

/// The args the command line set (or defaulted), typed as declared; never
/// the host's own options.
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
    use specforge_test_macros::test as specforge_test;

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
                    "order",
                    CommandArgType::Enum {
                        values: vec!["asc".into(), "desc".into()],
                    },
                    false,
                    Some("asc"),
                ),
            ],
            sandbox: None,
        }
    }

    fn parse_with_format(
        argv: &[&str],
    ) -> Result<(Map<String, Value>, CommandFormat), clap::Error> {
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
        Ok((arg_values(&c.args, sub), format_value(sub)))
    }

    fn parse(argv: &[&str]) -> Result<Map<String, Value>, clap::Error> {
        parse_with_format(argv).map(|(args, _)| args)
    }

    #[test]
    fn declared_args_become_typed_values() {
        let args = parse(&["milestone-completion", "m1", "--limit", "3", "--all-kinds"]).unwrap();
        assert_eq!(
            Value::Object(args),
            serde_json::json!({"milestone": "m1", "limit": 3, "all_kinds": true, "order": "asc"})
        );
    }

    #[specforge_test(
        behavior = "surface_format_conventions",
        verify = "default format is human"
    )]
    fn every_command_takes_the_hosts_format_human_by_default() {
        let (args, format) = parse_with_format(&["milestone-completion", "m1"]).unwrap();
        assert_eq!(format, CommandFormat::Human);
        assert!(!args.contains_key(FORMAT), "the format is not an arg");
        let (args, format) =
            parse_with_format(&["milestone-completion", "m1", "--format", "json"]).unwrap();
        assert_eq!(format, CommandFormat::Json);
        assert!(!args.contains_key(FORMAT));
        // There are no other formats.
        for other in ["table", "brief", "xml"] {
            assert!(parse(&["milestone-completion", "m1", "--format", other]).is_err());
        }
    }

    #[test]
    fn the_command_line_refuses_what_the_declaration_does() {
        // A required arg is positional and required; an enum takes its values.
        assert!(parse(&["milestone-completion"]).is_err());
        assert!(parse(&["milestone-completion", "m1", "--order", "sideways"]).is_err());
        assert!(parse(&["milestone-completion", "m1", "--limit", "many"]).is_err());
    }

    #[test]
    fn a_required_bool_is_a_flag_and_a_repeated_flag_is_refused() {
        let mut c = contribution();
        c.args.push(arg("strict", CommandArgType::Bool, true, None));
        let commands = [ExtensionCommand {
            extension: "@acme/x",
            contribution: &c,
        }];
        let parse = |argv: &[&str]| {
            command_line("x", &commands)
                .try_get_matches_from(["specforge x", "milestone-completion"].iter().chain(argv))
        };
        let matches = parse(&["m1", "--strict"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(arg_values(&c.args, sub)["strict"], true);
        let matches = parse(&["m1"]).unwrap();
        let (_, sub) = matches.subcommand().unwrap();
        assert_eq!(arg_values(&c.args, sub)["strict"], false);
        assert!(parse(&["m1", "--limit", "1", "--limit", "2"]).is_err());
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "a command declaring an arg named format is refused on the command line"
    )]
    fn an_arg_taking_a_host_option_or_another_args_name_is_refused() {
        let with = |args: Vec<CommandArg>| CommandContribution {
            args,
            ..contribution()
        };
        assert_eq!(refusal(&contribution()), None);
        for name in ["path", "format", "help"] {
            let c = with(vec![arg(name, CommandArgType::String, false, None)]);
            assert_eq!(
                refusal(&c),
                Some(format!("its arg '{name}' takes the host's --{name}"))
            );
        }
        let twice = with(vec![
            arg("all_kinds", CommandArgType::Bool, false, None),
            arg("all-kinds", CommandArgType::Bool, false, None),
        ]);
        assert_eq!(
            refusal(&twice),
            Some("it declares the arg 'all-kinds' twice".into())
        );
    }

    #[specforge_test(
        behavior = "dispatch_surface_command",
        verify = "under --format json a command whose export trapped prints one JSON error object"
    )]
    fn a_trapped_command_reports_in_the_format_asked_for() {
        let trap = specforge_common::Diagnostic::error(
            "E028",
            "CLI command cmd__x() trapped: unreachable: the command panicked",
        );
        let json = failed_run(&trap, CommandFormat::Json);
        let error: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            error,
            serde_json::json!({"code": "E028",
                "message": "CLI command cmd__x() trapped: unreachable: the command panicked"})
        );
        assert!(json.ends_with('\n') && json.lines().count() == 1, "{json}");
        let human = failed_run(&trap, CommandFormat::Human);
        assert!(
            human.starts_with("error[E028]: CLI command cmd__x() trapped"),
            "{human}"
        );
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
