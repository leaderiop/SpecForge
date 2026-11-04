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
    /// The commits of a range, newest first, with their parents, people, message and trailers, and how many
    Log(LogArgs),
    /// The files that differ between a commit and another, the index or the worktree, with their line counts
    Diff(DiffArgs),
    /// The nearest commit two revisions share
    MergeBase(MergeBaseArgs),
    /// A file at a revision (`HEAD:path`): its blob, size and text
    Show(ShowArgs),
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

#[derive(clap::Args)]
pub(crate) struct LogArgs {
    /// `A..B` (what B reaches and A does not), `A...B` (what either reaches and not both) or one revision;
    /// `HEAD` when absent
    range: Option<String>,

    /// At most this many commits
    #[arg(long, value_name = "N")]
    max: Option<usize>,

    /// Only the commits that change these paths, from the repository's top (after `--`, as git takes them)
    #[arg(last = true, value_name = "PATH")]
    paths: Vec<String>,

    /// Follow only each commit's first parent
    #[arg(long)]
    first_parent: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct DiffArgs {
    /// The commit to compare from; `HEAD` when absent
    #[arg(long, value_name = "REV")]
    base: Option<String>,

    /// The commit to compare with; `HEAD` when `--base` is given, else the worktree
    #[arg(long, value_name = "REV", conflicts_with_all = ["staged", "worktree"])]
    head: Option<String>,

    /// Compare with the index
    #[arg(long, conflicts_with = "worktree")]
    staged: bool,

    /// Compare with the tracked files as they are on disk
    #[arg(long)]
    worktree: bool,

    /// Only the files at or under these paths, from the repository's top (after `--`, as git takes them)
    #[arg(last = true, value_name = "PATH")]
    paths: Vec<String>,

    /// Give each text file's unified hunks
    #[arg(long)]
    patch: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct MergeBaseArgs {
    a: String,
    b: String,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ShowArgs {
    /// `<rev>:<path>`, the path from the repository's top
    object: String,

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
            Action::Log(args) => {
                let request = git::LogRequest {
                    range: args.range,
                    paths: args.paths,
                    first_parent: args.first_parent,
                    max: args.max,
                };
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::log(v, &request),
                    |t, a| {
                        for commit in &a.commits {
                            outln!(t, "{} {}", commit.sha.short(), shown(&commit.subject));
                        }
                    },
                )
            }
            Action::Diff(args) => {
                let head = if args.staged {
                    git::DiffHead::Staged
                } else if args.worktree {
                    git::DiffHead::Worktree
                } else {
                    git::DiffHead::Revision(args.head)
                };
                let request = git::DiffRequest {
                    base: args.base,
                    head,
                    paths: args.paths,
                    patch: args.patch,
                };
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::diff(v, &request),
                    |t, a| {
                        for file in &a.files {
                            let counts = if file.binary {
                                "-\t-".to_string()
                            } else {
                                format!("{}\t{}", file.added, file.removed)
                            };
                            outln!(t, "{counts}\t{}", shown(&file.path));
                        }
                    },
                )
            }
            Action::MergeBase(args) => {
                let (a, b) = (args.a, args.b);
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::merge_base(v, &a, &b),
                    |t, answer| {
                        if let Some(sha) = &answer.sha {
                            outln!(t, "{sha}");
                        }
                    },
                )
            }
            Action::Show(args) => {
                let object = args.object;
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::show(v, &object),
                    |t, a| match &a.text {
                        Some(text) => outln!(
                            t,
                            "{}",
                            specforge_common::remedy::printable_lines(text.trim_end_matches('\n'))
                        ),
                        None => outln!(t, "{} ({} bytes, binary)", a.sha, a.bytes),
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
