//! The verbs a person or an agent types (ADR 0339): `specforge files`, `run`, `http` and `git`. Each turns its
//! arguments into one `specforge_ops::verbs` operation and prints its answer: JSON by default, the values `--pick`
//! reaches, or a short listing under `--format human`; a refusal is the JSON error document (ADR 0029).

pub(crate) mod files;

use std::path::PathBuf;

use specforge_effects::Ports;
use specforge_ops::OpError;
use specforge_ops::consent::UserConsent;
use specforge_ops::verbs::Verbs;

use crate::outcome::{Exit, OutputFormat, Refusal};
use crate::pick::{self, Pick};
use crate::terminal::{Terminal, errln};

/// What every verb's leaf takes: the project, the output format (json by default) and `--pick`.
#[derive(clap::Args, Clone, Debug)]
pub(crate) struct VerbArgs {
    #[command(flatten)]
    project: crate::project::ProjectArg,

    /// Output format: json (the default) or human
    #[arg(long, default_value = "json")]
    format: OutputFormat,

    #[arg(long, value_name = "PATH", value_parser = Pick::parse, help = pick::HELP, long_help = pick::VERB_LONG_HELP)]
    pick: Option<Pick>,
}

impl VerbArgs {
    /// The output format, or the usage error (exit 2) of `--pick` beside `--format human`.
    pub(crate) fn format(&self, terminal: &Terminal) -> Result<OutputFormat, Exit> {
        match (&self.pick, self.format) {
            (Some(_), OutputFormat::Human) => {
                errln!(
                    terminal,
                    "error: --pick reads the JSON answer: it takes no --format human"
                );
                Err(Exit::Unjudged)
            }
            (_, format) => Ok(format),
        }
    }

    /// The verbs of the project at `--path`, typed in the current directory, through `ports`.
    pub(crate) fn open(&self, ports: &Ports) -> Result<Verbs, OpError> {
        let root = self.project.root()?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let consent = UserConsent::user(root.dir());
        Ok(Verbs::open(&root, &cwd, ports, &consent))
    }

    /// Print `answer` as the format asks (`human` prints it with `human`), or report its refusal; `failed` decides
    /// whether a printed answer still exits 1.
    pub(crate) fn print<T: serde::Serialize>(
        &self,
        terminal: &Terminal,
        format: OutputFormat,
        answer: Result<T, OpError>,
        human: impl FnOnce(&Terminal, &T),
    ) -> Exit {
        match answer {
            Ok(answer) => {
                match format {
                    OutputFormat::Json => pick::print_json(terminal, self.pick.as_ref(), &answer),
                    OutputFormat::Human => human(terminal, &answer),
                }
                Exit::Passed
            }
            Err(error) => Refusal::of(format).report(terminal, &error),
        }
    }
}

/// `text` from a file, a repository or a server, as a terminal may show it (ADR 0230).
pub(crate) fn shown(text: &str) -> std::borrow::Cow<'_, str> {
    specforge_common::remedy::printable(text)
}
