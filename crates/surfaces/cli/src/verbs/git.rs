//! `specforge git` (ADR 0339): the `Git` port for people and agents, in process, on the repository the project is in:
//! reads, and writes under the ceiling's `git` `write`.

use specforge_effects::Ports;
use specforge_effects::git::DiffStatus;
use specforge_ops::OpError;
use specforge_ops::verbs::{Verbs, git};

mod args;

use super::{VerbArgs, shown};
use crate::outcome::Exit;
use crate::terminal::{Terminal, errln, outln};
use args::*;

/// `specforge git [--in <dir>] <action>`: with `--in`, the action runs as if typed in `dir`, as `git -C` runs one
/// (its current directory is `dir`), once the roots of the project it is typed in hold `dir` as a typed path.
pub(crate) fn run(
    dir: Option<&std::path::Path>,
    action: Action,
    terminal: &Terminal,
    ports: &Ports,
) -> Exit {
    if let Some(dir) = dir {
        let held = super::open(&crate::project::ProjectArg::scan(&[]), ports)
            .and_then(|verbs| verbs.held(dir, specforge_effects::FsOp::Stat))
            .map(|(located, _)| located.at);
        let at = match held {
            Ok(at) => at,
            Err(error) => {
                let error = error.with_suggestion(format!(
                    "run it from that directory's own project: cd {} && specforge git {}",
                    shown(&dir.to_string_lossy()),
                    shown(&without_in().join(" "))
                ));
                return super::refusal(crate::outcome::OutputFormat::Human, true)
                    .report(terminal, &error);
            }
        };
        if let Err(error) = std::env::set_current_dir(&at) {
            errln!(
                terminal,
                "error: --in {}: {error}",
                shown(&dir.to_string_lossy())
            );
            return Exit::Unjudged;
        }
    }
    action.run(terminal, ports)
}

/// The words after `git` on this command line, `--in` and its directory left out.
fn without_in() -> Vec<String> {
    let mut words = std::env::args().skip_while(|word| word != "git").skip(1);
    let mut kept = Vec::new();
    while let Some(word) = words.next() {
        if word == "--in" {
            words.next();
        } else if !word.starts_with("--in=") {
            kept.push(word);
        }
    }
    kept
}

#[derive(clap::Subcommand)]
pub(crate) enum Action {
    /// Where the repository is: its top directory, git directory and common directory, the current directory's prefix
    /// and the repository it is a submodule of
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
    /// The submodules `.gitmodules` declares: each name, path and URL
    Submodules(Plain),
    /// Stage files as the worktree has them; ignore rules kept, a filter driver refused
    Add(AddArgs),
    /// Take files out of the index and off the disk (--cached: the index alone); a changed one needs -f
    Rm(RmArgs),
    /// Make a directory a repository, unless it is one
    Init(InitArgs),
    /// Run one of the repository's hooks
    #[command(subcommand)]
    Hook(HookAction),
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
    /// Apply a commit onto HEAD and record it with its author and message (or stage it, -n); conflicts exit 1
    CherryPick(PickArgs),
    /// Record a commit that undoes another (or stage its undoing, -n); conflicts exit 1
    Revert(PickArgs),
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
                let mut revs = args.revs;
                if args.stdin {
                    use std::io::BufRead as _;
                    let lines = std::io::stdin().lock().lines().map_while(Result::ok);
                    revs.extend(lines.filter(|line| !line.trim().is_empty()));
                }
                let order = match (args.sorted, args.reverse) {
                    (false, _) => None,
                    (true, reverse) => Some(git::RevOrder { reverse }),
                };
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::revs(v, &revs, order),
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
                    // The word the JSON answer says (`untracked`), not the Rust variant's name.
                    let kind = serde_json::to_value(entry.kind).expect("a unit variant serializes");
                    let kind = kind
                        .as_str()
                        .expect("a unit variant serializes as a string");
                    outln!(t, "{kind} {}", shown(&entry.path));
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
                    reverse: args.reverse,
                    grep: args.grep,
                    since: args.since,
                    until: args.until,
                    no_merges: args.no_merges,
                    pickaxe: args.pickaxe,
                    pickaxe_regex: args.pickaxe_regex,
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
                    merge_base: args.merge_base,
                };
                let shape = (args.patch, args.stat, args.shortstat);
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::diff(v, &request),
                    |t, a| diff_human(t, a, shape),
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
            Action::Submodules(args) => {
                answer(&args.verb, terminal, ports, git::submodules, |t, a| {
                    for module in &a.submodules {
                        outln!(t, "{}", shown(&module.path));
                    }
                })
            }
            Action::Remotes(args) => answer(&args.verb, terminal, ports, git::remotes, |t, a| {
                for remote in &a.remotes {
                    outln!(t, "{} {}", shown(&remote.name), shown(&remote.url));
                }
            }),
            Action::Add(args) => {
                let request = git::AddRequest {
                    paths: args.paths,
                    all: args.all,
                    except: args.except,
                    force: args.force,
                };
                // as git add: silent when it staged
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::add(v, &request),
                    |_, _| {},
                )
            }
            Action::Rm(args) => {
                let removal = git::Removal {
                    recursive: args.recursive,
                    cached: args.cached,
                    force: args.force,
                    ignore_unmatch: args.ignore_unmatch,
                };
                answer(
                    &args.verb,
                    terminal,
                    ports,
                    |v| git::rm(v, &args.paths, removal),
                    |t, a| {
                        for path in &a.removed {
                            outln!(t, "rm '{}'", shown(path));
                        }
                    },
                )
            }
            Action::Init(args) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::init(v, args.dir.as_deref()),
                |t, a| {
                    let made = if a.created {
                        "Initialized empty"
                    } else {
                        "Reinitialized existing"
                    };
                    outln!(t, "{made} Git repository in {}/.git/", shown(&a.top));
                },
            ),
            Action::Hook(HookAction::Run(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::hook_run(v, &args.name, &args.args, args.ignore_missing),
                |_, _| {},
            ),
            Action::Commit(args) => {
                let request = git::CommitRequest {
                    message: args.message.join("\n\n"),
                    reuse_message: args.reuse_message,
                    only: args.only,
                    add: args.add,
                    remove: args.remove,
                    all: args.all,
                    force: args.force,
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
            Action::Worktree(WorktreeAction::Lock(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::worktree_lock(v, &args.dir, true, args.reason.as_deref()),
                |_, _| {},
            ),
            Action::Worktree(WorktreeAction::Unlock(args)) => answer(
                &args.verb,
                terminal,
                ports,
                |v| git::worktree_lock(v, &args.dir, false, None),
                |_, _| {},
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
                let request = git::MergeRequest {
                    fast_forward: mode,
                    message: args.message,
                    no_commit: args.no_commit,
                    squash: args.squash,
                };
                merged(&args.verb, terminal, ports, |v| {
                    git::merge(v, &args.revision, &request)
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
                |v| {
                    let side = match (args.ours, args.theirs) {
                        (true, _) => Some(git::ConflictSide::Ours),
                        (_, true) => Some(git::ConflictSide::Theirs),
                        _ => None,
                    };
                    let request = git::RestoreRequest {
                        source: args.source.clone(),
                        staged: args.staged,
                        worktree: args.worktree,
                        side,
                    };
                    git::restore(v, &args.paths, &request)
                },
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
            Action::CherryPick(args) => picked(args, false, terminal, ports),
            Action::Revert(args) => picked(args, true, terminal, ports),
        }
    }
}

/// A cherry-pick or a revert (`revert`), or the end of the one in progress (`--quit`, `--abort`).
fn picked(args: PickArgs, revert: bool, terminal: &Terminal, ports: &Ports) -> Exit {
    match args.revision {
        Some(revision) => merged(&args.verb, terminal, ports, |v| {
            git::pick(v, &revision, revert, !args.no_commit)
        }),
        None => answer(
            &args.verb,
            terminal,
            ports,
            |v| git::pick_end(v, revert, args.abort),
            |_, _| {},
        ),
    }
}

/// A diff as people read it: a line per file (`--numstat`'s) unless only patches are asked, git's summary
/// under `--stat` or alone under `--shortstat`, and each file's patch under `--patch`.
fn diff_human(t: &Terminal, a: &git::DiffAnswer, (patch, stat, shortstat): (bool, bool, bool)) {
    let plural = |n: u64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    if !shortstat && (stat || !patch) {
        for file in &a.files {
            let counts = if file.binary {
                "-\t-".to_string()
            } else {
                format!("{}\t{}", file.added, file.removed)
            };
            outln!(t, "{counts}\t{}", shown(&file.path));
        }
    }
    if (stat || shortstat) && a.count > 0 {
        // git's own wording: insertions when there are any or no deletions, deletions likewise
        let mut summary = format!(" {} changed", plural(a.count as u64, "file", "files"));
        if a.added > 0 || a.removed == 0 {
            summary += &format!(", {}(+)", plural(a.added, "insertion", "insertions"));
        }
        if a.removed > 0 || a.added == 0 {
            summary += &format!(", {}(-)", plural(a.removed, "deletion", "deletions"));
        }
        outln!(t, "{summary}");
    }
    if !patch {
        return;
    }
    for file in &a.files {
        let old = file.old_path.as_deref().unwrap_or(&file.path);
        let side = |prefix: &str, path: &str, absent: bool| match absent {
            true => "/dev/null".to_string(),
            false => format!("{prefix}{}", shown(path)),
        };
        let (from, to) = (
            side("a/", old, file.status == DiffStatus::Added),
            side("b/", &file.path, file.status == DiffStatus::Deleted),
        );
        outln!(t, "diff --git a/{} b/{}", shown(old), shown(&file.path));
        if file.binary {
            outln!(t, "Binary files {from} and {to} differ");
            continue;
        }
        outln!(t, "--- {from}\n+++ {to}");
        if let Some(text) = &file.patch {
            outln!(
                t,
                "{}",
                specforge_common::remedy::printable_lines(text.trim_end_matches('\n'))
            );
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
    verb.answer(terminal, ports, |verbs| ask(&verbs), human)
}
