//! `specforge files` (ADR 0339): the `Fs` port for people and agents, in the project's roots.

use std::path::PathBuf;

use specforge_effects::Ports;
use specforge_effects::fs::FileKind;
use specforge_ops::OpError;
use specforge_ops::verbs::Verbs;
use specforge_ops::verbs::files::{self, ChangeAnswer, LIST_MAX, List, SEARCH_MAX, Search, Temp};

use super::{VerbArgs, shown};
use crate::outcome::Exit;
use crate::terminal::{Terminal, outln};

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// Each path's kind, size and modification time, or that it does not exist
    Stat(StatArgs),
    /// The files and directories under a directory, honouring .gitignore
    List(ListArgs),
    /// The lines under the paths that match a regular expression
    Search(SearchArgs),
    /// A new empty file, or directory, with a name no other has, in the project's temp root (only you may read it)
    Temp(TempArgs),
    /// Make each directory, with its missing parents
    Mkdir(PathsArgs),
    /// Copy a file, or a directory and what it holds, onto a path or into a directory
    Copy(PairArgs),
    /// Move a file or directory onto a path or into a directory
    Move(PairArgs),
    /// Remove each file, or empty directory (with --recursive, a directory and what it holds)
    Remove(RemoveArgs),
    /// The SHA-256 of each file, and of --text
    Digest(DigestArgs),
}

#[derive(clap::Args)]
pub(crate) struct TempArgs {
    /// Make a directory, not a file
    #[arg(long)]
    dir: bool,

    /// The name's start
    #[arg(long, default_value = "")]
    prefix: String,

    /// The name's end, such as `.json`
    #[arg(long, default_value = "")]
    suffix: String,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct PathsArgs {
    /// The paths, from the current directory
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct PairArgs {
    /// What to copy or move, from the current directory
    from: PathBuf,

    /// Where to, from the current directory: into it when it is a directory
    to: PathBuf,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct RemoveArgs {
    /// The paths, from the current directory
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Remove a directory and what it holds
    #[arg(long, short = 'r')]
    recursive: bool,

    /// A path that does not exist is no error, as `rm -f`
    #[arg(long, short = 'f')]
    force: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct DigestArgs {
    /// The files, from the current directory
    #[arg(required_unless_present = "text")]
    paths: Vec<PathBuf>,

    /// Digest this text too (its UTF-8 bytes)
    #[arg(long, value_name = "TEXT")]
    text: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct StatArgs {
    /// The paths, from the current directory
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    #[command(flatten)]
    verb: VerbArgs,
}

/// What `--kind` lists.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum Kind {
    File,
    Dir,
}

#[derive(clap::Args)]
pub(crate) struct ListArgs {
    /// The directory, from the current directory
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Only entries that match one of these globs (gitignore's): one with a `/` is matched against the path below the
    /// directory, one without against a name at any depth; `!GLOB` leaves a file or a whole directory out
    #[arg(long = "glob", value_name = "GLOB")]
    globs: Vec<String>,

    /// Only files, or only directories
    #[arg(long, value_enum)]
    kind: Option<Kind>,

    /// How deep below the directory to list (1: its own entries)
    #[arg(long, value_name = "N")]
    depth: Option<usize>,

    /// List what .gitignore and .ignore leave out too
    #[arg(long)]
    no_ignore: bool,

    /// List names that start with `.`
    #[arg(long)]
    hidden: bool,

    /// The most entries listed
    #[arg(long, value_name = "N", default_value_t = LIST_MAX)]
    max: usize,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct SearchArgs {
    /// The regular expression (Rust's `regex` syntax)
    pattern: String,

    /// The files and directories to search, from the current directory (the current directory when none)
    paths: Vec<PathBuf>,

    /// Only files whose name (or path below a searched directory, for a glob with a `/`) matches one of these globs
    /// (gitignore's); `!GLOB` leaves a file or a whole directory out
    #[arg(long = "glob", value_name = "GLOB")]
    globs: Vec<String>,

    /// Match letters in any case
    #[arg(long, short = 'i')]
    ignore_case: bool,

    /// The pattern is a literal string, not a regular expression
    #[arg(long, short = 'F')]
    fixed_strings: bool,

    /// Answer the lines that do not match
    #[arg(long, short = 'v')]
    invert_match: bool,

    /// A match may span lines: the pattern may hold `\n` (and `\s` then matches one)
    #[arg(long, short = 'U')]
    multiline: bool,

    /// Answer N lines after each match, in its `after`
    #[arg(long, short = 'A', value_name = "N")]
    after_context: Option<usize>,

    /// Answer N lines before each match, in its `before`
    #[arg(long, short = 'B', value_name = "N")]
    before_context: Option<usize>,

    /// Answer N lines before and after each match (-A and -B win over it)
    #[arg(long, short = 'C', value_name = "N")]
    context: Option<usize>,

    /// The most matches answered
    #[arg(long, value_name = "N", default_value_t = SEARCH_MAX)]
    max: usize,

    #[command(flatten)]
    verb: VerbArgs,
}

impl Action {
    pub(crate) fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        match self {
            Action::Stat(args) => args.verb.answer(
                terminal,
                ports,
                |verbs| files::stat(&verbs, &args.paths),
                |terminal, answer| {
                    for entry in &answer.entries {
                        let what = match (entry.kind, entry.size) {
                            (Some(kind), Some(size)) => format!("{} {size} B", kind.name()),
                            (Some(kind), None) => kind.name().to_string(),
                            (None, _) => "missing".to_string(),
                        };
                        outln!(terminal, "{}  {what}", shown(&entry.path));
                    }
                },
            ),
            Action::List(args) => args.run(terminal, ports),
            Action::Search(args) => args.run(terminal, ports),
            Action::Temp(args) => {
                let temp = Temp {
                    dir: args.dir,
                    prefix: args.prefix,
                    suffix: args.suffix,
                };
                args.verb.answer(
                    terminal,
                    ports,
                    |verbs| files::temp(verbs, &temp),
                    |terminal, answer| outln!(terminal, "{}", shown(&answer.path)),
                )
            }
            Action::Mkdir(args) => changes(&args.verb, terminal, ports, |verbs| {
                files::mkdir(verbs, &args.paths)
            }),
            Action::Copy(args) => changes(&args.verb, terminal, ports, |verbs| {
                files::copy(verbs, &args.from, &args.to)
            }),
            Action::Move(args) => changes(&args.verb, terminal, ports, |verbs| {
                files::move_to(verbs, &args.from, &args.to)
            }),
            Action::Remove(args) => changes(&args.verb, terminal, ports, |verbs| {
                files::remove(verbs, &args.paths, args.recursive, args.force)
            }),
            Action::Digest(args) => args.verb.answer(
                terminal,
                ports,
                |verbs| files::digest(&verbs, &args.paths, args.text.as_deref()),
                |terminal, answer| {
                    for digest in &answer.digests {
                        let path = digest.path.as_deref().unwrap_or("--text");
                        outln!(terminal, "{}  {}", digest.sha256, shown(path));
                    }
                },
            ),
        }
    }
}

/// A change's answer: each path it made, wrote or removed.
fn changes(
    verb: &VerbArgs,
    terminal: &Terminal,
    ports: &Ports,
    change: impl FnOnce(&Verbs) -> Result<ChangeAnswer, OpError>,
) -> Exit {
    verb.answer(
        terminal,
        ports,
        |verbs| change(&verbs),
        |terminal, answer| {
            for path in &answer.changed {
                outln!(terminal, "{}", shown(path));
            }
        },
    )
}

impl ListArgs {
    fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        let list = List {
            dir: self.dir,
            globs: self.globs,
            kind: self.kind.map(|kind| match kind {
                Kind::File => FileKind::File,
                Kind::Dir => FileKind::Dir,
            }),
            depth: self.depth,
            ignore: !self.no_ignore,
            hidden: self.hidden,
            max: self.max,
        };
        self.verb.answer(
            terminal,
            ports,
            |verbs| files::list(&verbs, &list),
            |terminal, answer| {
                for entry in &answer.entries {
                    let slash = if entry.kind == FileKind::Dir { "/" } else { "" };
                    outln!(terminal, "{}{slash}", shown(&entry.path));
                }
                if answer.truncated {
                    outln!(terminal, "(stopped at {} entries)", answer.count);
                }
            },
        )
    }
}

impl SearchArgs {
    fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        let search = Search {
            pattern: self.pattern,
            paths: self.paths,
            globs: self.globs,
            ignore_case: self.ignore_case,
            fixed: self.fixed_strings,
            invert: self.invert_match,
            multiline: self.multiline,
            before: self.before_context.or(self.context).unwrap_or(0),
            after: self.after_context.or(self.context).unwrap_or(0),
            max: self.max,
        };
        self.verb.answer(
            terminal,
            ports,
            |verbs| files::search(&verbs, &search),
            |terminal, answer| {
                let path = |found: &files::Match| shown(&found.path).into_owned();
                for found in &answer.matches {
                    for line in found.before.iter().flatten() {
                        outln!(
                            terminal,
                            "{}-{}- {}",
                            path(found),
                            line.line,
                            shown(&line.text)
                        );
                    }
                    outln!(
                        terminal,
                        "{}:{}:{}: {}",
                        path(found),
                        found.line,
                        found.column,
                        shown(&found.text)
                    );
                    for line in found.after.iter().flatten() {
                        outln!(
                            terminal,
                            "{}-{}- {}",
                            path(found),
                            line.line,
                            shown(&line.text)
                        );
                    }
                }
                if answer.truncated {
                    outln!(terminal, "(stopped at {} matches)", answer.count);
                }
            },
        )
    }
}
