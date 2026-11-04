//! `specforge git` (ADR 0339): the `Git` port for people and agents, in process, on the repository the project is in:
//! reads, and writes under the ceiling's `git` `write`.

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
    /// The local branches with their upstream and how far each is ahead and behind, or the remote-tracking ones
    Branches(BranchesArgs),
    /// The main worktree and the linked ones, with the branch each is on
    Worktrees(Plain),
    /// The remotes, with each URL's host and `owner/repo` path
    Remotes(Plain),
    /// Record a commit: stage the paths named, run the repository's hooks, commit by git's configured identity
    Commit(CommitArgs),
    /// Create or delete a branch
    #[command(subcommand)]
    Branch(BranchAction),
    /// Move HEAD to a branch, its files checked out; a file the worktree changed is never overwritten
    Switch(SwitchArgs),
    /// Create a tag: annotated with -m, else a plain ref
    Tag(TagArgs),
    /// Add, remove or prune linked worktrees
    #[command(subcommand)]
    Worktree(WorktreeAction),
    /// Merge a revision into HEAD: a fast-forward when history allows, else a merge commit; conflicts exit 1
    Merge(MergeArgs),
    /// Move HEAD to a revision, and the index (--mixed, the default) and the files (--hard) with it
    Reset(ResetArgs),
    /// Put files back as the index has them (or --source), discarding their changes; --staged: their index entries
    Restore(RestoreArgs),
    /// Set the worktree's changes aside, bring them back, or list them
    #[command(subcommand)]
    Stash(StashAction),
    /// Apply a commit onto HEAD and record it with its author and message; conflicts exit 1
    CherryPick(PickArgs),
    /// Record a commit that undoes another; conflicts exit 1
    Revert(PickArgs),
}

#[derive(clap::Args)]
pub(crate) struct MergeArgs {
    /// The branch or commit merged in
    revision: String,

    /// Refuse unless HEAD can fast-forward
    #[arg(long, conflicts_with = "no_ff")]
    ff_only: bool,

    /// Record a merge commit even when HEAD could fast-forward
    #[arg(long)]
    no_ff: bool,

    /// The merge commit's message
    #[arg(short, long, value_name = "MESSAGE")]
    message: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct ResetArgs {
    /// Where HEAD goes; HEAD when absent
    revision: Option<String>,

    /// Move HEAD alone
    #[arg(long, conflicts_with_all = ["mixed", "hard"])]
    soft: bool,

    /// Move HEAD and the index (the default)
    #[arg(long, conflicts_with = "hard")]
    mixed: bool,

    /// Move HEAD, the index and the files: changes are lost
    #[arg(long)]
    hard: bool,

    /// Discard the tree's changes with --hard
    #[arg(long)]
    yes: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct RestoreArgs {
    /// The files, from the current directory
    #[arg(required = true)]
    paths: Vec<std::path::PathBuf>,

    /// Take them from this commit instead
    #[arg(long, value_name = "REV")]
    source: Option<String>,

    /// Restore their index entries (from HEAD, or --source), not the files
    #[arg(long)]
    staged: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum StashAction {
    /// Stash the changes of the index and the worktree, and clean the tree
    Push(StashPushArgs),
    /// Apply a stash (the newest when absent) and drop it
    Pop(StashPopArgs),
    /// The stashes, newest first
    List(Plain),
}

#[derive(clap::Args)]
pub(crate) struct StashPushArgs {
    #[arg(short, long, value_name = "MESSAGE")]
    message: Option<String>,

    /// Stash the untracked files too
    #[arg(short = 'u', long)]
    include_untracked: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct StashPopArgs {
    /// The stash's index (`0` the newest)
    #[arg(default_value_t = 0)]
    index: usize,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct PickArgs {
    /// The commit
    revision: String,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum BranchAction {
    /// Create a branch at a revision (HEAD when absent)
    Create(BranchCreateArgs),
    /// Delete a branch: never HEAD's, and one with commits HEAD does not hold only with --force
    Delete(BranchDeleteArgs),
}

#[derive(clap::Args)]
pub(crate) struct BranchCreateArgs {
    name: String,

    /// Where the branch starts; HEAD when absent
    #[arg(long, value_name = "REV")]
    at: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct BranchDeleteArgs {
    name: String,

    /// Delete it even when HEAD does not hold its commits
    #[arg(long)]
    force: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct SwitchArgs {
    /// The branch
    name: String,

    /// Create the branch first (at --at, or HEAD)
    #[arg(short, long)]
    create: bool,

    /// Where a created branch starts; HEAD when absent
    #[arg(long, value_name = "REV", requires = "create")]
    at: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct TagArgs {
    name: String,

    /// The commit tagged; HEAD when absent
    #[arg(long, value_name = "REV")]
    at: Option<String>,

    /// An annotated tag's message, by git's configured identity
    #[arg(short, long, value_name = "MESSAGE")]
    message: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Subcommand)]
pub(crate) enum WorktreeAction {
    /// Add a linked worktree at a directory (absent or empty) on a branch: its name when no --branch, made when missing
    Add(WorktreeAddArgs),
    /// Remove a linked worktree and its directory: one with changes or untracked files only with --force
    Remove(WorktreeRemoveArgs),
    /// Drop the records of linked worktrees whose directories are gone
    Prune(Plain),
}

#[derive(clap::Args)]
pub(crate) struct WorktreeAddArgs {
    /// The worktree's directory, from the current directory
    #[arg(value_name = "DIR")]
    dir: std::path::PathBuf,

    /// The branch it is on
    #[arg(long, value_name = "NAME")]
    branch: Option<String>,

    /// Create the branch first (at --at, or HEAD)
    #[arg(short, long)]
    create: bool,

    /// Where a created branch starts; HEAD when absent
    #[arg(long, value_name = "REV")]
    at: Option<String>,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct WorktreeRemoveArgs {
    /// The worktree's directory, from the current directory
    #[arg(value_name = "DIR")]
    dir: std::path::PathBuf,

    /// Remove it with its changes and untracked files
    #[arg(long)]
    force: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct CommitArgs {
    /// The message; given more than once, each is a paragraph of its own (as `git commit -m` takes them)
    #[arg(short, long = "message", value_name = "MESSAGE", required = true)]
    message: Vec<String>,

    /// Stage the files at or under this path first, as the worktree has them (a deleted one leaves the index)
    #[arg(long, value_name = "PATH")]
    add: Vec<std::path::PathBuf>,

    /// Take the files at or under this path out of the index first, leaving them on disk (`git rm --cached`)
    #[arg(long, value_name = "PATH")]
    remove: Vec<std::path::PathBuf>,

    /// Stage every tracked file first as the worktree has it; untracked files stay out
    #[arg(short, long)]
    all: bool,

    /// Commit even when the index matches HEAD
    #[arg(long)]
    allow_empty: bool,

    /// Run none of the repository's hooks (pre-commit, commit-msg, post-commit)
    #[arg(long)]
    no_verify: bool,

    #[command(flatten)]
    verb: VerbArgs,
}

#[derive(clap::Args)]
pub(crate) struct BranchesArgs {
    /// The remote-tracking branches instead (`origin/main`)
    #[arg(long)]
    remote: bool,

    #[command(flatten)]
    verb: VerbArgs,
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

    /// At most this many commits listed (1000 when absent; `count` still counts them all, and `truncated` says so)
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
            Action::Branches(args) => {
                let remote = args.remote;
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::branches(v, remote),
                    |t, a| {
                        for branch in &a.branches {
                            let mark = if branch.current { "*" } else { " " };
                            outln!(t, "{mark} {}", shown(&branch.name));
                        }
                    },
                )
            }
            Action::Worktrees(args) => {
                answer(&args.verb, terminal, ports, git::worktrees, |t, a| {
                    for worktree in &a.worktrees {
                        let branch = worktree.branch.as_deref().unwrap_or("(detached)");
                        outln!(t, "{} {}", shown(&worktree.path), shown(branch));
                    }
                })
            }
            Action::Remotes(args) => answer(&args.verb, terminal, ports, git::remotes, |t, a| {
                for remote in &a.remotes {
                    outln!(t, "{} {}", shown(&remote.name), shown(&remote.url));
                }
            }),
            Action::Commit(args) => {
                let request = git::CommitRequest {
                    message: args.message.join("\n\n"),
                    add: args.add,
                    remove: args.remove,
                    all: args.all,
                    allow_empty: args.allow_empty,
                    no_verify: args.no_verify,
                };
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::commit(v, &request),
                    |t, a| {
                        let on = a.branch.as_deref().unwrap_or("detached HEAD");
                        outln!(
                            t,
                            "[{} {}] {} files",
                            shown(on),
                            &a.sha[..a.sha.len().min(7)],
                            a.files.len()
                        );
                    },
                )
            }
            Action::Branch(BranchAction::Create(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::branch_create(v, &args.name, args.at.as_deref()),
                |t, a| outln!(t, "{} {}", shown(&a.name), a.sha),
            ),
            Action::Branch(BranchAction::Delete(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::branch_delete(v, &args.name, args.force),
                |t, a| outln!(t, "deleted {} (was {})", shown(&a.name), a.sha),
            ),
            Action::Switch(args) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::switch(v, &args.name, args.create, args.at.as_deref()),
                |t, a| outln!(t, "{}", shown(a.branch.as_deref().unwrap_or("(detached)"))),
            ),
            Action::Tag(args) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::tag(v, &args.name, args.at.as_deref(), args.message.as_deref()),
                |t, a| outln!(t, "{} {}", shown(&a.name), a.sha),
            ),
            Action::Worktree(WorktreeAction::Add(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| {
                    git::worktree_add(
                        v,
                        &args.dir,
                        args.branch.as_deref(),
                        args.create,
                        args.at.as_deref(),
                    )
                },
                |t, a| {
                    outln!(
                        t,
                        "{} {}",
                        shown(&a.path),
                        shown(a.branch.as_deref().unwrap_or("(detached)"))
                    )
                },
            ),
            Action::Worktree(WorktreeAction::Remove(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::worktree_remove(v, &args.dir, args.force),
                |t, a| {
                    for path in &a.pruned {
                        outln!(t, "removed {}", shown(path));
                    }
                },
            ),
            Action::Worktree(WorktreeAction::Prune(args)) => {
                answer(&args.verb, terminal, ports, git::worktree_prune, |t, a| {
                    for path in &a.pruned {
                        outln!(t, "pruned {}", shown(path));
                    }
                })
            }
            Action::Merge(args) => {
                let mode = if args.ff_only {
                    git::FastForward::Only
                } else if args.no_ff {
                    git::FastForward::Never
                } else {
                    git::FastForward::Allowed
                };
                merged(&args.verb, terminal, ports, |v| {
                    git::merge(v, &args.revision, mode, args.message.as_deref())
                })
            }
            Action::Reset(args) => {
                let mode = if args.soft {
                    git::ResetMode::Soft
                } else if args.hard {
                    git::ResetMode::Hard
                } else {
                    git::ResetMode::Mixed
                };
                let revision = args.revision.unwrap_or_else(|| "HEAD".to_string());
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::reset(v, &revision, mode, args.yes),
                    |t, a| outln!(t, "HEAD is now at {}", &a.sha[..a.sha.len().min(7)]),
                )
            }
            Action::Restore(args) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::restore(v, &args.paths, args.source.as_deref(), args.staged),
                |t, a| {
                    for path in &a.restored {
                        outln!(t, "{}", shown(path));
                    }
                },
            ),
            Action::Stash(StashAction::Push(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::stash_push(v, args.message.as_deref(), args.include_untracked),
                |t, a| outln!(t, "stashed {}", &a.sha[..a.sha.len().min(7)]),
            ),
            Action::Stash(StashAction::Pop(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::stash_pop(v, args.index),
                |t, a| outln!(t, "{} stashes left", a.stashes.len()),
            ),
            Action::Stash(StashAction::List(args)) => {
                answer(&args.verb, terminal, ports, git::stash_list, |t, a| {
                    for stash in &a.stashes {
                        outln!(t, "stash@{{{}}}: {}", stash.index, shown(&stash.message));
                    }
                })
            }
            Action::CherryPick(args) => merged(&args.verb, terminal, ports, |v| {
                git::pick(v, &args.revision, false)
            }),
            Action::Revert(args) => merged(&args.verb, terminal, ports, |v| {
                git::pick(v, &args.revision, true)
            }),
        }
    }
}

/// A merge's, a cherry-pick's or a revert's answer: printed, and exit 1 when it stopped on conflicts.
fn merged(
    verb: &VerbArgs,
    terminal: &Terminal,
    ports: &Ports,
    ask: impl FnOnce(&Verbs) -> Result<git::MergeAnswer, OpError>,
) -> Exit {
    let mut conflicted = false;
    let exit = answer(
        verb,
        terminal,
        ports,
        |v| {
            let merged = ask(v)?;
            conflicted = !merged.conflicts.is_empty();
            Ok(merged)
        },
        |t, a| match &a.sha {
            Some(sha) => outln!(t, "HEAD is now at {}", &sha[..sha.len().min(7)]),
            None => {
                for conflict in &a.conflicts {
                    outln!(t, "conflict ({}): {}", conflict.kind, shown(&conflict.path));
                }
            }
        },
    );
    if conflicted && exit == Exit::Passed {
        Exit::Failed
    } else {
        exit
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
