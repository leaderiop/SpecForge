//! `specforge files` (ADR 0339): the `Fs` port for people and agents, in the project's roots.

use std::path::PathBuf;

use specforge_effects::Ports;
use specforge_effects::fs::FileKind;
use specforge_ops::OpError;
use specforge_ops::verbs::Verbs;
use specforge_ops::verbs::files::{
    self, ChangeAnswer, LIST_MAX, List, SEARCH_BYTES, SEARCH_MAX, Search, SearchMode, Stdin, Temp,
    UnreadWhy,
};

use super::{VerbArgs, shown};
use crate::outcome::Exit;
use crate::terminal::{Terminal, out, outln};

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// Each path's kind, size and modification time, or that it does not exist
    Stat(StatArgs),
    /// The files and directories under a directory, honouring .gitignore
    List(ListArgs),
    /// The lines under the paths that match a regular expression
    Search(SearchArgs),
    /// A file's text (at most 1 MiB), how many bytes were read and its lines
    Read(ReadArgs),
    /// A new empty file, or directory, with a name no other has (or the one --name gives), in the project's temp root
    /// (only you may read it)
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

    /// One stable name, no separator: the same path every call, made when missing and kept when it is there
    #[arg(long, conflicts_with_all = ["prefix", "suffix"])]
    name: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ReadArgs {
    /// The file, from the current directory
    file: PathBuf,

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

    /// Count each file's lines (a last line with no newline counts) and answer their total in `lines`
    #[arg(long)]
    lines: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct SearchArgs {
    /// The regular expression (Rust's `regex` syntax)
    pattern: String,

    /// The files and directories to search, from the current directory (the current directory when none); `-` is
    /// standard input, read only when typed
    paths: Vec<PathBuf>,

    /// Only files whose name (or path below a searched directory, for a glob with a `/`) matches one of these globs
    /// (gitignore's); `!GLOB` leaves a file or a whole directory out
    #[arg(long = "glob", value_name = "GLOB")]
    globs: Vec<String>,

    /// Match letters in any case
    #[arg(long, short = 'i')]
    ignore_case: bool,

    /// Search what .gitignore and .ignore leave out too
    #[arg(long)]
    no_ignore: bool,

    /// Search names that start with `.` (a file the host withholds, such as `.env`, is still never read)
    #[arg(long)]
    hidden: bool,

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

    /// Answer the files that match, not their lines (each file's search stops at its first match)
    #[arg(long, short = 'l', conflicts_with_all = ["counts", "after_context", "before_context", "context"])]
    files_with_matches: bool,

    /// Answer each file's count of matching lines: every file with a match, and every file typed by name, even at 0
    #[arg(long, conflicts_with_all = ["after_context", "before_context", "context"])]
    counts: bool,

    /// Print nothing: exit 0 when anything matched, else 2 when a typed path is missing, a file went unread or the
    /// search is refused (its error on stderr), else 1, as `grep -q`; it takes no --pick and no --format
    #[arg(long, short = 'q', conflicts_with = "none", conflicts_with_all = QUIET_CONFLICTS)]
    quiet: bool,

    /// Print nothing: exit 1 when anything matched, else 2 when a typed path is missing, a file went unread or the
    /// search is refused (its error on stderr), else 0: a negative gate that never passes on a file it could not read
    #[arg(long, conflicts_with_all = QUIET_CONFLICTS)]
    none: bool,

    /// Leave out a line (or -U match) this pattern also matches, as `| grep -v`; it takes -i and -F too, and context
    /// lines are kept
    #[arg(long = "and-not", value_name = "REGEX")]
    and_not: Vec<String>,

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
            Action::Read(args) => args.verb.answer(
                terminal,
                ports,
                |verbs| files::read(&verbs, &args.file),
                |terminal, answer| {
                    out!(
                        terminal,
                        "{}",
                        specforge_common::remedy::printable_lines(&answer.text)
                    );
                },
            ),
            Action::Temp(args) => {
                let temp = Temp {
                    dir: args.dir,
                    prefix: args.prefix,
                    suffix: args.suffix,
                    name: args.name,
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
            lines: self.lines,
        };
        self.verb.answer(
            terminal,
            ports,
            |verbs| files::list(&verbs, &list),
            |terminal, answer| {
                for entry in &answer.entries {
                    let slash = if entry.kind == FileKind::Dir { "/" } else { "" };
                    match entry.lines {
                        Some(Some(lines)) => outln!(terminal, "{}  {lines}", shown(&entry.path)),
                        Some(None) => outln!(terminal, "{}  ?", shown(&entry.path)),
                        None => outln!(terminal, "{}{slash}", shown(&entry.path)),
                    }
                }
                if let Some(lines) = answer.lines {
                    outln!(terminal, "{lines} lines");
                }
                if answer.truncated {
                    outln!(terminal, "(stopped at {} entries)", answer.count);
                }
            },
        )
    }
}

/// Standard input, at most [`SEARCH_BYTES`], and why that is not all of it.
fn stdin_bytes() -> Stdin {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    let read = std::io::stdin()
        .lock()
        .take(SEARCH_BYTES + 1)
        .read_to_end(&mut bytes);
    let cut = if read.is_err() {
        Some(UnreadWhy::Unreadable)
    } else if bytes.len() as u64 > SEARCH_BYTES {
        Some(UnreadWhy::TooLarge)
    } else {
        None
    };
    bytes.truncate(SEARCH_BYTES as usize);
    Stdin { bytes, cut }
}

/// What `-q` and `--none` take no part of: an answer they do not print.
const QUIET_CONFLICTS: [&str; 8] = [
    "files_with_matches",
    "counts",
    "after_context",
    "before_context",
    "context",
    crate::pick::PICK,
    "format",
    "max",
];

impl SearchArgs {
    fn run(self, terminal: &Terminal, ports: &Ports) -> Exit {
        let verdict = self.quiet || self.none;
        // Read once, and only when `-` is typed: no search waits on a terminal or an agent's open stdin.
        let stdin = if self.paths.iter().any(|path| path.as_os_str() == "-") {
            stdin_bytes()
        } else {
            Stdin::default()
        };
        let search = Search {
            pattern: self.pattern,
            paths: self.paths,
            globs: self.globs,
            ignore: !self.no_ignore,
            hidden: self.hidden,
            ignore_case: self.ignore_case,
            fixed: self.fixed_strings,
            invert: self.invert_match,
            multiline: self.multiline,
            before: self.before_context.or(self.context).unwrap_or(0),
            after: self.after_context.or(self.context).unwrap_or(0),
            and_not: self.and_not,
            mode: if self.files_with_matches || verdict {
                SearchMode::Files
            } else if self.counts {
                SearchMode::Counts
            } else {
                SearchMode::Lines
            },
            // A verdict is judged at the first match.
            max: if verdict { 1 } else { self.max },
            stdin,
        };
        if verdict {
            let quiet = self.quiet;
            return self.verb.verdict(
                terminal,
                ports,
                |verbs| files::search(&verbs, &search),
                |answer| {
                    if answer.count > 0 {
                        Exit::of_verdict(quiet)
                    } else if answer.missing.is_empty() && answer.unread.is_empty() {
                        Exit::of_verdict(!quiet)
                    } else {
                        Exit::Unjudged
                    }
                },
            );
        }
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
                for file in answer.files.iter().flatten() {
                    outln!(terminal, "{}", shown(file));
                }
                for counted in answer.counts.iter().flatten() {
                    outln!(terminal, "{}:{}", shown(&counted.path), counted.count);
                }
                for path in &answer.missing {
                    outln!(terminal, "(missing: {})", shown(path));
                }
                for unread in &answer.unread {
                    outln!(
                        terminal,
                        "(unread: {}: {})",
                        shown(&unread.path),
                        unread.why.name()
                    );
                }
                if answer.truncated {
                    outln!(terminal, "(stopped at {} answered)", answer.count);
                }
            },
        )
    }
}
