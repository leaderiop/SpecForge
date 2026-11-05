//! `specforge git branch`, `switch`, `tag` and `worktree` (ADR 0339 D5): refs written by gix, a checkout and a
//! worktree by libgit2, each under the ceiling's `git` `write`. A switch overwrites no file the worktree changed, and a
//! worktree with changes is not removed unless forced.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specforge_common::remedy::printable;
use specforge_effects::RealFs;
use specforge_effects::git::{GitPlace, Located, Sha};

use super::commit::{Signed, held, refuse_signing, write_failed};
use super::{HeadAnswer, Worktree, commit_named, failed, head, slashed};
use crate::verbs::Verbs;
use crate::{OpError, OpErrorKind};

/// A branch or tag the verb wrote or deleted, and the commit it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefAnswer {
    pub name: String,
    pub sha: String,
}

/// `git worktree prune`'s answer: the worktrees whose records were dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrunedAnswer {
    pub pruned: Vec<String>,
}

fn refused(code: &'static str, message: String, suggestion: &str) -> OpError {
    OpError::new(OpErrorKind::PreconditionFailed, code, message).with_suggestion(suggestion)
}

/// `git init`'s answer: the repository's top and whether it was made (false: one was there).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InitAnswer {
    pub top: String,
    pub created: bool,
}

/// `git init [<dir>]`: the directory (read from the current directory, held to the project's roots, made by gix
/// when missing) made a repository unless it holds one.
pub fn init(verbs: &Verbs, dir: Option<&Path>) -> Result<InitAnswer, OpError> {
    verbs.permit_git_write("init")?;
    let at = held(verbs, dir.unwrap_or(Path::new(".")))?;
    let created = verbs.ports().git.init(&at).map_err(write_failed)?;
    Ok(InitAnswer {
        top: slashed(&at),
        created,
    })
}

/// `git worktree lock` and `unlock`'s answer: the worktree and whether it is locked now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LockAnswer {
    pub path: String,
    pub locked: bool,
}

/// Lock the linked worktree at `path` (read from the current directory) against a prune or a removal, `reason` kept
/// with it, or unlock it (`lock` false).
pub fn worktree_lock(
    verbs: &Verbs,
    path: &Path,
    lock: bool,
    reason: Option<&str>,
) -> Result<LockAnswer, OpError> {
    let place = verbs.place_to_write(if lock {
        "worktree lock"
    } else {
        "worktree unlock"
    })?;
    let at = landed(verbs, path);
    verbs
        .ports()
        .git
        .worktree_lock(&place, &at, lock, reason)
        .map_err(write_failed)?;
    Ok(LockAnswer {
        path: slashed(&at),
        locked: lock,
    })
}

/// The branch `HEAD` is on, when one is.
fn current(verbs: &Verbs, place: &GitPlace) -> Result<Option<String>, OpError> {
    Ok(verbs.ports().git.head(place).map_err(failed)?.head)
}

pub fn branch_create(verbs: &Verbs, name: &str, start: Option<&str>) -> Result<RefAnswer, OpError> {
    let place = verbs.place_to_write("branch create")?;
    let sha = commit_named(verbs, &place, start.unwrap_or("HEAD"))?;
    verbs
        .ports()
        .git
        .branch_create(&place, name, &sha)
        .map_err(write_failed)?;
    Ok(RefAnswer {
        name: name.to_string(),
        sha: sha.as_str().to_string(),
    })
}

/// Delete the branch `name`: never the one `HEAD` is on, and one whose commits `HEAD` does not hold only when
/// `force` (`git branch -d` and `-D`).
pub fn branch_delete(verbs: &Verbs, name: &str, force: bool) -> Result<RefAnswer, OpError> {
    let place = verbs.place_to_write("branch delete")?;
    if current(verbs, &place)?.as_deref() == Some(name) {
        return Err(refused(
            "current_branch",
            format!("HEAD is on {}: it cannot be deleted", printable(name)),
            "switch to another branch first",
        ));
    }
    let git = &verbs.ports().git;
    let sha = commit_named(verbs, &place, &format!("refs/heads/{name}"))?;
    if !force {
        let merged = match git.commit(&place, "HEAD").map_err(failed)? {
            Some(head) => git.merge_base(&place, &sha, &head).map_err(failed)? == Some(sha.clone()),
            None => false,
        };
        if !merged {
            return Err(refused(
                "not_merged",
                format!("{} holds commits HEAD does not", printable(name)),
                "merge it first, or run specforge git branch delete --force to delete it with them",
            ));
        }
    }
    git.branch_delete(&place, name).map_err(write_failed)?;
    Ok(RefAnswer {
        name: name.to_string(),
        sha: sha.as_str().to_string(),
    })
}

/// Move `HEAD` to the branch `name`, its files checked out; with `create`, the branch is made first at `start`
/// (`HEAD` when absent). A file the worktree changed that the switch would overwrite refuses it, and nothing moves.
pub fn switch(
    verbs: &Verbs,
    name: &str,
    create: bool,
    start: Option<&str>,
) -> Result<HeadAnswer, OpError> {
    let place = verbs.place_to_write("switch")?;
    let git = &verbs.ports().git;
    if create {
        let sha = commit_named(verbs, &place, start.unwrap_or("HEAD"))?;
        git.branch_create(&place, name, &sha)
            .map_err(write_failed)?;
        if let Err(error) = git.switch(&place, name) {
            // nothing moved: the branch made for it goes too
            let _ = git.branch_delete(&place, name);
            return Err(write_failed(error));
        }
    } else {
        git.switch(&place, name).map_err(write_failed)?;
    }
    head(verbs)
}

pub fn tag(
    verbs: &Verbs,
    name: &str,
    at: Option<&str>,
    message: Option<&str>,
) -> Result<RefAnswer, OpError> {
    let place = verbs.place_to_write("tag")?;
    if message.is_some() {
        refuse_signing(verbs, &place, Signed::Tag)?;
    }
    let sha = commit_named(verbs, &place, at.unwrap_or("HEAD"))?;
    let tagged = verbs
        .ports()
        .git
        .tag(&place, name, &sha, message)
        .map_err(write_failed)?;
    Ok(RefAnswer {
        name: name.to_string(),
        sha: tagged.as_str().to_string(),
    })
}

/// `path` read from the current directory, where it lands.
fn landed(verbs: &Verbs, path: &Path) -> PathBuf {
    RealFs::landing(&verbs.cwd().join(path))
}

/// Add a linked worktree at `path` (read from the current directory, absent or empty) on the branch `branch`, made at
/// `start` first when `create`. With no branch, the branch is the directory's name, made at `HEAD` when it does not
/// exist (as `git worktree add <path>` does).
pub fn worktree_add(
    verbs: &Verbs,
    path: &Path,
    branch: Option<&str>,
    create: bool,
    start: Option<&str>,
) -> Result<Worktree, OpError> {
    let place = verbs.place_to_write("worktree add")?;
    let at = held(verbs, path)?;
    if std::fs::read_dir(&at).is_ok_and(|mut entries| entries.next().is_some()) {
        return Err(refused(
            "path_taken",
            format!("{} already holds files", printable(&at.to_string_lossy())),
            "name a directory that does not exist yet, or an empty one",
        ));
    }
    let git = &verbs.ports().git;
    let named = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let branch = branch.map(str::to_string).or(named).ok_or_else(|| {
        OpError::new(
            OpErrorKind::InvalidInput,
            OpErrorKind::InvalidInput.as_str(),
            "name the worktree's directory",
        )
    })?;
    let exists = git
        .commit(&place, &format!("refs/heads/{branch}"))
        .map_err(failed)?
        .is_some();
    if create || !exists {
        let sha = commit_named(verbs, &place, start.unwrap_or("HEAD"))?;
        git.branch_create(&place, &branch, &sha)
            .map_err(write_failed)?;
    }
    git.worktree_add(&place, &at, &branch)
        .map_err(write_failed)?;
    let top = std::fs::canonicalize(&at).unwrap_or(at);
    let added = git
        .worktrees(&place)
        .map_err(failed)?
        .into_iter()
        .find(|w| w.path == top)
        .ok_or_else(|| {
            write_failed(specforge_effects::git::GitError(
                "the new worktree is not listed".into(),
            ))
        })?;
    Ok(Worktree {
        path: slashed(&added.path),
        branch: added.branch,
        sha: added.sha.map(|sha: Sha| sha.as_str().to_string()),
        main: added.main,
        locked: added.locked,
        current: false,
    })
}

/// Remove the linked worktree at `path` (held to the project's roots), its directory with it: one with a change or an
/// untracked file only when `force` (`git worktree remove --force`).
pub fn worktree_remove(verbs: &Verbs, path: &Path, force: bool) -> Result<PrunedAnswer, OpError> {
    let place = verbs.place_to_write("worktree remove")?;
    // its files are deleted: held to the roots, as `worktree add` holds where it writes
    let at = held(verbs, path)?;
    let git = &verbs.ports().git;
    if !force {
        let top = std::fs::canonicalize(&at).ok();
        match git.locate(&at) {
            Located::Repo(there) if Some(&there.toplevel) == top.as_ref() => {
                if git.status(&there).map_err(failed)?.dirty() {
                    return Err(refused(
                        super::DIRTY_TREE,
                        format!(
                            "the worktree at {} has changes or untracked files",
                            printable(&at.to_string_lossy())
                        ),
                        "commit or stash them, or run specforge git worktree remove --force to remove them with it",
                    ));
                }
            }
            // a directory that is gone is only a record to drop, as git's `worktree remove` takes it
            _ if top.is_none() => {}
            // git validates the worktree first: a broken `.git` leaves its files unknown, so they may be work
            _ => {
                return Err(refused(
                    "broken_worktree",
                    format!(
                        "{} is not a working worktree (its .git is missing or broken)",
                        printable(&at.to_string_lossy())
                    ),
                    "run specforge git worktree remove --force to remove it with its files",
                ));
            }
        }
    }
    git.worktree_remove(&place, &at).map_err(write_failed)?;
    Ok(PrunedAnswer {
        pruned: vec![slashed(&at)],
    })
}

/// Drop the records of the linked worktrees whose directories are gone.
pub fn worktree_prune(verbs: &Verbs) -> Result<PrunedAnswer, OpError> {
    let place = verbs.place_to_write("worktree prune")?;
    let pruned = verbs
        .ports()
        .git
        .worktree_prune(&place)
        .map_err(write_failed)?;
    Ok(PrunedAnswer {
        pruned: pruned.iter().map(|p| slashed(p)).collect(),
    })
}
