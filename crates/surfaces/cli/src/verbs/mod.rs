//! The verbs a person or an agent types (ADR 0339): `specforge files`, `run`, `http` and `git`. Each turns its
//! arguments into one `specforge_ops::verbs` operation and prints its answer: JSON by default, the values `--pick`
//! reaches, or a short listing under `--format human`; a refusal is the JSON error document (ADR 0029).

pub(crate) mod files;
pub(crate) mod git;
pub(crate) mod http;
pub(crate) mod run;

use std::path::PathBuf;
use std::time::Duration;

use specforge_effects::{EffectsMode, Ports};
use specforge_ops::OpError;
use specforge_ops::consent::UserConsent;
use specforge_ops::verbs::{Answered, Verbs};

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

    #[command(flatten)]
    shape: pick::Shape,

    /// The effects mode: online (the default; auto is online), cached or replay. http sends the request, answers a
    /// GET from a fresh record of the effects journal (cached), or answers from the journal alone (replay); files and
    /// git read and write this machine's disk in every mode, and the journal keeps none of it
    #[arg(long, value_enum)]
    effects: Option<EffectsMode>,
}

impl VerbArgs {
    /// The effects mode `--effects` names, `online` when none.
    pub(crate) fn mode(&self) -> EffectsMode {
        self.effects.unwrap_or(EffectsMode::Online)
    }

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

    /// `ask` of the verbs at `--path`, a local read or write (`files`, `git`) performed in every effects mode, printed
    /// as the format asks with its `effect`.
    pub(crate) fn answer<T: serde::Serialize>(
        &self,
        terminal: &Terminal,
        ports: &Ports,
        ask: impl FnOnce(Verbs) -> Result<T, OpError>,
        human: impl FnOnce(&Terminal, &T),
    ) -> Exit {
        self.answered(
            terminal,
            ports,
            |verbs| ask(verbs).map(Answered::performed),
            human,
        )
    }

    /// [`Self::answer`] for a verb whose answer says how it was had (`http`, through the effects journal).
    pub(crate) fn answered<T: serde::Serialize>(
        &self,
        terminal: &Terminal,
        ports: &Ports,
        ask: impl FnOnce(Verbs) -> Result<Answered<T>, OpError>,
        human: impl FnOnce(&Terminal, &T),
    ) -> Exit {
        let format = match self.format(terminal) {
            Ok(format) => format,
            Err(exit) => return exit,
        };
        let answer = open(&self.project, ports).and_then(ask);
        self.print(terminal, format, answer, |terminal, answered| {
            human(terminal, &answered.answer);
        })
    }

    /// Print `answer` as the format asks (`human` prints it with `human`), or report its refusal; `failed` decides
    /// whether a printed answer still exits 1.
    fn print<T: serde::Serialize>(
        &self,
        terminal: &Terminal,
        format: OutputFormat,
        answer: Result<T, OpError>,
        human: impl FnOnce(&Terminal, &T),
    ) -> Exit {
        match answer {
            Ok(answer) => {
                match format {
                    OutputFormat::Json => {
                        pick::print_json(terminal, self.pick.as_ref(), &self.shape, &answer)
                    }
                    OutputFormat::Human => human(terminal, &answer),
                }
                Exit::Passed
            }
            Err(error) => refusal(format, self.pick.is_some()).report(terminal, &error),
        }
    }
}

/// The verbs of the project `project` names, typed in the current directory, through `ports`.
pub(crate) fn open(project: &crate::project::ProjectArg, ports: &Ports) -> Result<Verbs, OpError> {
    let root = project.root()?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let consent = UserConsent::user(root.dir());
    Ok(Verbs::open(&root, &cwd, ports, &consent))
}

/// How a verb reports a refusal: the JSON error document on stdout, or with `--pick` `error[CODE]` on stderr, so a
/// shell that reads the picked values reads nothing (ADR 0339 D4).
pub(crate) fn refusal(format: OutputFormat, pick: bool) -> Refusal<'static> {
    if pick {
        Refusal::fixed()
    } else {
        Refusal::of(format)
    }
}

/// `text` from a file, a repository or a server, as a terminal may show it (ADR 0230).
pub(crate) fn shown(text: &str) -> std::borrow::Cow<'_, str> {
    specforge_common::remedy::printable(text)
}

/// A duration as `timeout` reads one: `10` (seconds) or jiff's friendly form, `10s`, `500ms`, `2m`, `1h`, `1h30m`.
pub(crate) fn parse_duration(text: &str) -> Result<Duration, String> {
    let refused = || format!("{text:?} is no duration: write 10, 10s, 500ms, 2m or 1h");
    // A bare number is seconds, as `timeout` reads one; jiff reads every form with a unit.
    if let Ok(seconds) = text.parse::<u64>() {
        return Ok(Duration::from_secs(seconds));
    }
    let signed: jiff::SignedDuration = text.parse().map_err(|_| refused())?;
    Duration::try_from(signed).map_err(|_| refused())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duration_reads_like_timeout() {
        assert_eq!(parse_duration("10"), Ok(Duration::from_secs(10)));
        assert_eq!(parse_duration("10s"), Ok(Duration::from_secs(10)));
        assert_eq!(parse_duration("500ms"), Ok(Duration::from_millis(500)));
        assert_eq!(parse_duration("2m"), Ok(Duration::from_secs(120)));
        assert_eq!(parse_duration("1h"), Ok(Duration::from_secs(3600)));
        assert_eq!(parse_duration("1h30m"), Ok(Duration::from_secs(5400)));
        assert!(parse_duration("1d").is_err());
        assert!(parse_duration("s").is_err());
        assert!(parse_duration("-5s").is_err());
    }
}
