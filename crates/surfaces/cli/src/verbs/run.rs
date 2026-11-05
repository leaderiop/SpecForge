//! `specforge run` (ADR 0339): the one runner for people and agents. Its exit is the program's, or 124 at its deadline,
//! 127 when not found and 126 when it cannot start; `--format json` captures the run instead, and `--which` finds
//! programs without running any.

use std::time::Duration;

use specforge_effects::{EffectsMode, Ports};
use specforge_ops::verbs::Answered;
use specforge_ops::verbs::run::{self as ran, Run};

use super::shown;
use crate::outcome::{Exit, OutputFormat};
use crate::pick::{self, Pick};
use crate::terminal::{Terminal, errln, outln};

#[derive(clap::Args)]
pub(crate) struct RunArgs {
    /// The program, then its arguments (after `--` when one starts with `-`)
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_name = "PROGRAM",
        required = true
    )]
    argv: Vec<String>,

    /// Stop the program and its children after this long and exit 124: `10` or `10s`, `500ms`, `2m`, `1h` (24h when
    /// absent, the most)
    #[arg(long, value_name = "DURATION", value_parser = super::parse_duration)]
    deadline: Option<Duration>,

    /// Run the program in this directory (from the current directory, in the project's roots), with `PWD` naming it
    #[arg(long = "in", value_name = "DIR")]
    dir: Option<std::path::PathBuf>,

    /// Print where the runner finds each program, and run none
    #[arg(long)]
    which: bool,

    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Output format: human (the default) passes the program's stdin, stdout and stderr through; json captures them,
    /// with a null stdin and at most 64 MiB of each stream. --pick implies json, so it captures too
    #[arg(long, default_value = "human")]
    format: OutputFormat,

    #[arg(long, value_name = "PATH", value_parser = Pick::parse, help = pick::HELP, long_help = pick::VERB_LONG_HELP)]
    pick: Option<Pick>,

    #[command(flatten)]
    shape: pick::Shape,

    /// With --format json: whether the run is performed (online, the default; auto is online) or answered from the
    /// effects journal (replay). A program asks for something to happen, so cached performs it too
    #[arg(long, value_enum)]
    effects: Option<EffectsMode>,
}

impl RunArgs {
    /// The exit code: the program's own, or the verb's.
    pub(crate) fn run(self, terminal: &Terminal, ports: &Ports) -> i32 {
        // --pick reads the JSON answer, so it asks for one: a pass-through run has none to pick from.
        let json = self.format == OutputFormat::Json || self.pick.is_some();
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Human
        };
        let mode = self.effects.unwrap_or(EffectsMode::Online);
        if !json && !self.which && !mode.is_live() {
            errln!(
                terminal,
                "error: a run that passes its output through keeps none, so nothing answers it again: --effects {} \
                 takes --format json",
                mode.name()
            );
            return Exit::Unjudged.code();
        }
        let verbs = match super::open(&self.project, ports) {
            Ok(verbs) => verbs,
            Err(error) => {
                return super::refusal(format, self.pick.is_some())
                    .report(terminal, &error)
                    .code();
            }
        };
        let run = Run {
            argv: self.argv.clone(),
            deadline: self.deadline,
            dir: self.dir.clone(),
        };
        if self.which {
            // A lookup on this machine, performed in every mode.
            let answer = Answered::performed(ran::which(&verbs, &self.argv));
            return match format {
                // The answer says what was found: printed, it is the run's success.
                OutputFormat::Json => {
                    pick::print_json(terminal, self.pick.as_ref(), &self.shape, &answer);
                    Exit::Passed.code()
                }
                // As `command -v`: the paths found, and 1 when one is missing.
                OutputFormat::Human => {
                    for program in &answer.answer.programs {
                        if let Some(path) = &program.path {
                            outln!(terminal, "{}", shown(path));
                        }
                    }
                    let all = answer.answer.programs.iter().all(|program| program.found);
                    if all { Exit::Passed } else { Exit::Failed }.code()
                }
            };
        }
        if json {
            return match ran::capture(&verbs, &run, mode) {
                Ok(answer) => {
                    pick::print_json(terminal, self.pick.as_ref(), &self.shape, &answer);
                    Exit::Passed.code()
                }
                Err(error) => super::refusal(format, self.pick.is_some())
                    .report(terminal, &error)
                    .code(),
            };
        }
        // What the program printed is on the terminal: flush ours first, so the two never interleave out of order.
        terminal.flush();
        match ran::pass_through(&verbs, &run) {
            Ok(class) => {
                let (_, code) = ran::ending(&class);
                if !matches!(class, specforge_effects::ProcessClass::Exited(_)) {
                    errln!(terminal, "specforge run: {} {class}", shown(&self.argv[0]));
                }
                code
            }
            Err(error) => super::refusal(format, self.pick.is_some())
                .report(terminal, &error)
                .code(),
        }
    }
}
