//! `specforge prompt <short>.<id> [arguments]`: render an extension's prompt (ADR 0291). The words after the name are
//! the prompt's arguments as its declaration reads them (`specforge_ops::extension::prompt::args`): a required
//! argument is positional, any other is `--<name> <value>`, a bool is `--<name>`. The host's own options, `--host`,
//! `--format` and `--path`, are read wherever they stand. `specforge <short> <id>` renders the same prompt when no
//! command of the extension has the name ([`crate::extension_command`]).

use crate::outcome::{Exit, OutputArg, OutputFormat, Refusal};
use crate::project::ProjectArg;
use crate::terminal::{Terminal, errln, outln};
use specforge_ops::OpErrorKind;
use specforge_ops::extension::prompt::{ExtensionPrompts, PromptArgs, RenderError, render};
use specforge_ops::host::Host;
use specforge_ops::view::ProjectView;
use specforge_project::CompiledProject;
use specforge_protocol_types::PromptDescriptor;

/// `specforge prompt`'s command line.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The prompt: the extension's short name, a dot and the prompt's id (`gsd.plan_phase`)
    name: String,

    /// The prompt's arguments, as `specforge <extension> <command>` takes a command's
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments: Vec<String>,

    /// The host to render for: claude-code, codex or cursor
    #[arg(long)]
    host: Option<String>,

    #[command(flatten)]
    project: ProjectArg,

    #[command(flatten)]
    output: OutputArg,
}

impl Args {
    pub(crate) fn run(self, terminal: &Terminal) -> Exit {
        let mut rest = self.arguments;
        // The options the host owns, wherever they stand: after the first argument they are the trailing words.
        let project = if rest
            .iter()
            .any(|w| w == "--path" || w.starts_with("--path="))
        {
            ProjectArg::scan(&rest)
        } else {
            self.project
        };
        let project = match project.compile() {
            Ok(project) => project,
            Err(error) => return Refusal::fixed().report(terminal, &error),
        };
        if let Some(host) = self.host {
            rest.extend(["--host".to_string(), host]);
        }
        run_in(&project, &self.name, rest, self.output.format, terminal)
    }
}

/// Remove `--<name> <value>` or `--<name>=<value>` from `words`, and give the value.
fn take(words: &mut Vec<String>, name: &str) -> Option<String> {
    let flag = format!("--{name}");
    let at = words
        .iter()
        .position(|w| *w == flag || w.starts_with(&format!("{flag}=")))?;
    let word = words.remove(at);
    match word.strip_prefix(&format!("{flag}=")) {
        Some(value) => Some(value.to_string()),
        None if at < words.len() => Some(words.remove(at)),
        None => None,
    }
}

/// `specforge prompt <name> <rest…>` over `project`. `name` is `<short>.<id>`.
pub(crate) fn run_in(
    project: &CompiledProject,
    name: &str,
    rest: Vec<String>,
    format: OutputFormat,
    terminal: &Terminal,
) -> Exit {
    let (short, id) = name.split_once('.').unwrap_or(("", name));
    served(project, short, id, rest, format, terminal)
}

/// The prompt of the extension `short` that `id` names (its id, or its command-line spelling), rendered with the
/// words `rest` and printed.
pub(crate) fn served(
    project: &CompiledProject,
    short: &str,
    id: &str,
    mut rest: Vec<String>,
    mut format: OutputFormat,
    terminal: &Terminal,
) -> Exit {
    let view = ProjectView::of(project);
    let prompts = ExtensionPrompts::build(&view);
    let cli_name = specforge_protocol_types::command_args::option_name(id);
    let Some(prompt) = prompts.by_cli_name(short, &cli_name) else {
        errln!(terminal, "error: no prompt '{short}.{id}' in the project");
        let served: Vec<String> = prompts
            .all()
            .iter()
            .map(|p| format!("{}.{}", p.short(), p.id()))
            .collect();
        if !served.is_empty() {
            errln!(terminal, "  prompts: {}", served.join(", "));
        }
        return Exit::Unjudged;
    };
    take(&mut rest, "path");
    if take(&mut rest, "format").is_some_and(|f| f == "json") {
        format = OutputFormat::Json;
    }
    let host = match take(&mut rest, "host") {
        Some(flag) => match Host::from_flag(&flag) {
            Some(host) => Some(host),
            None => {
                errln!(
                    terminal,
                    "error: unknown host '{flag}': claude-code, codex or cursor"
                );
                return Exit::Unjudged;
            }
        },
        None => None,
    };
    match render(&view, prompt, PromptArgs::Words(&rest), host) {
        Ok(rendered) => {
            match format {
                OutputFormat::Json => terminal.json(&rendered.document()),
                OutputFormat::Human => {
                    for message in &rendered.messages {
                        outln!(terminal, "{}", message.text);
                    }
                    outln!(
                        terminal,
                        "{}",
                        serde_json::to_string_pretty(&rendered.payload)
                            .expect("a payload serializes")
                    );
                }
            }
            Exit::Passed
        }
        Err(error) => failed(&error, format, terminal),
    }
}

/// A prompt that did not render, on stderr in the format asked for, and the exit code: 2 for an argument the
/// declaration refuses, as the arg rule of a command, else 1.
fn failed(error: &RenderError, format: OutputFormat, terminal: &Terminal) -> Exit {
    let (refusal, usage) = match error {
        RenderError::Refused(e) => (
            Some(e),
            matches!(
                e.kind,
                OpErrorKind::InvalidInput | OpErrorKind::EntityNotFound
            ),
        ),
        RenderError::Gate(e) => (Some(e), false),
        RenderError::Call(_) => (None, false),
    };
    let text = match (refusal, format) {
        (Some(e), OutputFormat::Json) => serde_json::json!({
            "code": e.code, "message": e.message, "suggestion": e.suggestion, "data": e.data,
        })
        .to_string(),
        (Some(e), OutputFormat::Human) => crate::extension_command::failed_gate(e),
        (None, _) => match error {
            RenderError::Call(call) => crate::extension_command::call_text(&call.diagnostic()),
            _ => unreachable!("only a call has no refusal"),
        },
    };
    errln!(terminal, "{}", text.trim_end());
    if usage { Exit::Unjudged } else { Exit::Failed }
}

/// The prompt `declared`, routed as `cli_name`, as a subcommand of its extension's command line: what completions and `--help` list. Its arguments are
/// the declaration's, each as a command's would be (a required one positional, a bool a flag, any other an option),
/// then the host's own `--host`, `--path` and `--format`.
pub(crate) fn subcommand(cli_name: &str, declared: &PromptDescriptor) -> clap::Command {
    use clap::{Arg, ArgAction};
    use specforge_protocol_types::command_args::option_name;
    let mut sub = clap::Command::new(cli_name.to_string())
        .about(format!("{} (prompt)", declared.title))
        .long_about(declared.description.clone());
    for arg in &declared.args {
        let mut clap_arg = Arg::new(arg.name.clone());
        clap_arg = if arg.field_type == "bool" {
            clap_arg
                .long(option_name(&arg.name))
                .action(ArgAction::SetTrue)
        } else if arg.required {
            clap_arg.value_name(arg.name.to_uppercase()).required(true)
        } else {
            clap_arg
                .long(option_name(&arg.name))
                .value_name(arg.name.to_uppercase())
        };
        if let Some(help) = &arg.description {
            clap_arg = clap_arg.help(help.clone());
        }
        sub = sub.arg(clap_arg);
    }
    sub.arg(
        Arg::new("host")
            .long("host")
            .value_name("HOST")
            .help("The host to render for: claude-code, codex or cursor"),
    )
    .arg(ProjectArg::arg())
    .arg(OutputArg::arg())
}
