//! `specforge git` (ADR 0339): the `Git` port for people and agents, read in process, on the repository the project
//! is in.

use specforge_effects::Ports;
use specforge_ops::OpError;
use specforge_ops::verbs::{Verbs, git};

use super::{VerbArgs, shown};
use crate::outcome::Exit;
use crate::terminal::{Terminal, outln};

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// Where the repository is: its top directory, git directory and common directory
    Repo(Plain),
    /// HEAD's commit and branch, whether it is detached or unborn, and its upstream
    Head(Plain),
    /// What each revision names: its sha and kind, or that it names nothing
    Rev(RevArgs),
    /// The working tree's status: each changed, conflicted and untracked path, and whether it is clean
    Status(Plain),
    /// Every value a configuration key has, and the one git applies
    Config(ConfigArgs),
}

#[derive(clap::Args)]
pub(crate) struct Plain {
    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct RevArgs {
    /// The revisions: a sha, a branch, a tag, `HEAD~1`, `main:path`
    #[arg(required = true)]
    revs: Vec<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ConfigArgs {
    /// The key, `section.name` or `section.subsection.name`
    key: String,

    #[command(flatten)]
    verb: VerbArgs,
}

impl Action {
    pub(crate) fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        match self {
            Action::Repo(args) => answer(&args.verb, terminal, ports, git::repo, |t, a| {
                outln!(t, "{}", shown(&a.top));
            }),
            Action::Head(args) => answer(&args.verb, terminal, ports, git::head, |t, a| {
                let at = a.sha.as_deref().unwrap_or("(unborn)");
                outln!(
                    t,
                    "{} {}",
                    shown(a.branch.as_deref().unwrap_or("(detached)")),
                    at
                );
            }),
            Action::Rev(args) => {
                let revs = args.revs;
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::revs(v, &revs),
                    |t, a| {
                        for rev in &a.revs {
                            outln!(
                                t,
                                "{} {}",
                                rev.sha.as_deref().unwrap_or("-"),
                                shown(&rev.rev)
                            );
                        }
                    },
                )
            }
            Action::Status(args) => answer(&args.verb, terminal, ports, git::status, |t, a| {
                for entry in &a.status.entries {
                    outln!(t, "{:?} {}", entry.kind, shown(&entry.path));
                }
            }),
            Action::Config(args) => {
                let key = args.key;
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::config(v, &key),
                    |t, a| {
                        if let Some(value) = &a.value {
                            outln!(t, "{}", shown(value));
                        }
                    },
                )
            }
        }
    }
}

/// One question's answer, printed as the format asks.
fn answer<T: serde::Serialize>(
    verb: &VerbArgs,
    terminal: &Terminal,
    ports: &Ports,
    ask: impl FnOnce(&Verbs) -> Result<T, OpError>,
    human: impl FnOnce(&Terminal, &T),
) -> Exit {
    let format = match verb.format(terminal) {
        Ok(format) => format,
        Err(exit) => return exit,
    };
    let answer = verb.open(ports).and_then(|verbs| ask(&verbs));
    verb.print(terminal, format, answer, human)
}
