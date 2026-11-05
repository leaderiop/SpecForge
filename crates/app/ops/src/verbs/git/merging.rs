//! `specforge git merge`, `reset`, `restore`, `stash`, `cherry-pick` and `revert` (ADR 0339 D5). Each asks the
//! ceiling's `git` `write`. A merge, a cherry-pick and a revert need a tree with no tracked change, and stop on
//! conflicts, which are answered and left in the files for the user to resolve; `reset --hard` over changes needs
//! `--yes`. No file the worktree changed is overwritten unless the user named it (`restore`) or said yes.

use std::path::PathBuf;

use serde::Serialize;
use specforge_common::remedy::printable;
use specforge_effects::git::{GitConflict, GitEntryKind, GitPlace, GitStash, Merged, ResetMode};

use super::commit::{Signed, in_repository, refuse_signing, write_failed};
use super::{commit_named, failed};
use crate::verbs::Verbs;
use crate::{OpError, OpErrorKind};

/// What a merge, a cherry-pick or a revert did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergeAnswer {
    /// The commit `HEAD` is on after it; none when it stopped on conflicts.
    pub sha: Option<String>,
    pub fast_forward: bool,
    pub conflicts: Vec<GitConflict>,
}

impl From<Merged> for MergeAnswer {
    fn from(merged: Merged) -> Self {
        MergeAnswer {
            sha: merged.sha.map(|sha| sha.as_str().to_string()),
            fast_forward: merged.fast_forward,
            conflicts: merged.conflicts,
        }
    }
}

/// `git reset`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResetAnswer {
    pub sha: String,
    pub mode: ResetMode,
}

/// `git restore`'s answer: the paths restored, from the repository's top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RestoreAnswer {
    pub restored: Vec<String>,
}

/// `git cherry-pick --quit` or `--abort`'s answer (`git revert`'s too).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PickEndAnswer {
    /// A cherry-pick or a revert was in progress and is over.
    pub ended: bool,
}

/// `git stash push`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StashAnswer {
    pub sha: String,
}

/// `git stash list`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StashesAnswer {
    pub stashes: Vec<GitStash>,
}

/// How `git merge` may move `HEAD`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FastForward {
    /// A fast-forward when history allows, else a merge commit.
    #[default]
    Allowed,
    /// A fast-forward or nothing (`--ff-only`).
    Only,
    /// Always a merge commit (`--no-ff`).
    Never,
}

/// Refuse when the tree has a tracked change or a conflict: a merge, a pick or a hard reset would mix it in or lose it.
fn clean_tree(
    verbs: &Verbs,
    place: &GitPlace,
    what: &str,
    suggestion: &str,
) -> Result<(), OpError> {
    let status = verbs.ports().git.status(place).map_err(failed)?;
    let tracked = status
        .entries
        .iter()
        .find(|e| !matches!(e.kind, GitEntryKind::Untracked | GitEntryKind::Ignored));
    match tracked {
        None => Ok(()),
        Some(entry) => Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            super::DIRTY_TREE,
            format!(
                "{} has changes ({} among them): {what} would mix them in or lose them",
                printable(&place.toplevel.to_string_lossy()),
                printable(&entry.path)
            ),
        )
        .with_suggestion(suggestion)),
    }
}

const COMMIT_OR_STASH: &str = "commit or stash the changes first";

pub fn merge(
    verbs: &Verbs,
    revision: &str,
    fast_forward: FastForward,
    message: Option<&str>,
) -> Result<MergeAnswer, OpError> {
    let place = verbs.place_to_write("merge")?;
    let theirs = commit_named(verbs, &place, revision)?;
    let git = &verbs.ports().git;
    let ours = git.commit(&place, "HEAD").map_err(failed)?;
    let base = match &ours {
        Some(ours) => git.merge_base(&place, ours, &theirs).map_err(failed)?,
        None => None,
    };
    // a merge commit is made unless `revision` is already in, or `HEAD` fast-forwards to it
    let up_to_date = base.as_ref() == Some(&theirs);
    let forwards = ours.is_none() || base == ours;
    let commits = !up_to_date
        && !matches!(
            (fast_forward, forwards),
            (FastForward::Only, _) | (FastForward::Allowed, true)
        );
    if commits {
        refuse_signing(verbs, &place, Signed::Commit)?;
    }
    clean_tree(verbs, &place, "a merge", COMMIT_OR_STASH)?;
    let branch = git.head(&place).map_err(failed)?.head;
    let message = match message {
        Some(message) => super::cleaned(message).ok_or_else(|| {
            OpError::new(
                OpErrorKind::InvalidInput,
                OpErrorKind::InvalidInput.as_str(),
                "the merge message is empty",
            )
        })?,
        None => match branch {
            Some(branch) if branch != "main" && branch != "master" => {
                format!("Merge {revision} into {branch}\n")
            }
            _ => format!("Merge {revision}\n"),
        },
    };
    let mode = match fast_forward {
        FastForward::Allowed => None,
        FastForward::Only => Some(true),
        FastForward::Never => Some(false),
    };
    verbs
        .ports()
        .git
        .merge(&place, &theirs, mode, &message)
        .map(MergeAnswer::from)
        .map_err(write_failed)
}

/// Move `HEAD` to `revision`: `hard` over a tree with changes only with `yes`, since they are lost.
pub fn reset(
    verbs: &Verbs,
    revision: &str,
    mode: ResetMode,
    yes: bool,
) -> Result<ResetAnswer, OpError> {
    let place = verbs.place_to_write("reset")?;
    if mode == ResetMode::Hard && !yes {
        clean_tree(
            verbs,
            &place,
            "a hard reset",
            "commit or stash them, or run specforge git reset --hard --yes to discard them",
        )?;
    }
    let to = commit_named(verbs, &place, revision)?;
    verbs
        .ports()
        .git
        .reset(&place, &to, mode)
        .map_err(write_failed)?;
    Ok(ResetAnswer {
        sha: to.as_str().to_string(),
        mode,
    })
}

/// The named files back as the index (or `source`) has them, their changes discarded; with `staged`, their index
/// entries back as `HEAD` (or `source`) has them.
pub fn restore(
    verbs: &Verbs,
    paths: &[PathBuf],
    source: Option<&str>,
    staged: bool,
) -> Result<RestoreAnswer, OpError> {
    let place = verbs.place_to_write("restore")?;
    let restored = in_repository(verbs, &place, paths)?;
    let source = source
        .map(|revision| commit_named(verbs, &place, revision))
        .transpose()?;
    verbs
        .ports()
        .git
        .restore(&place, &restored, source.as_ref(), staged)
        .map_err(write_failed)?;
    Ok(RestoreAnswer { restored })
}

pub fn stash_push(
    verbs: &Verbs,
    message: Option<&str>,
    untracked: bool,
) -> Result<StashAnswer, OpError> {
    let place = verbs.place_to_write("stash push")?;
    let sha = verbs
        .ports()
        .git
        .stash_push(&place, message, untracked)
        .map_err(write_failed)?;
    Ok(StashAnswer {
        sha: sha.as_str().to_string(),
    })
}

/// Apply the stash at `index` and drop it: the stashes left.
pub fn stash_pop(verbs: &Verbs, index: usize) -> Result<StashesAnswer, OpError> {
    let place = verbs.place_to_write("stash pop")?;
    let git = &verbs.ports().git;
    git.stash_pop(&place, index).map_err(write_failed)?;
    Ok(StashesAnswer {
        stashes: git.stashes(&place).map_err(failed)?,
    })
}

pub fn stash_list(verbs: &Verbs) -> Result<StashesAnswer, OpError> {
    let place = verbs.place("stash list")?;
    Ok(StashesAnswer {
        stashes: verbs.ports().git.stashes(&place).map_err(failed)?,
    })
}

/// Apply `revision` onto `HEAD` (`revert`: its inverse) and record it, or stop on conflicts. Unless `record`, it is
/// applied onto the index and left staged (`--no-commit`): the index may hold staged changes then, as git lets a
/// second `-n` stack on the first, but no file may differ from it.
pub fn pick(
    verbs: &Verbs,
    revision: &str,
    revert: bool,
    record: bool,
) -> Result<MergeAnswer, OpError> {
    let what = if revert { "revert" } else { "cherry-pick" };
    let place = verbs.place_to_write(what)?;
    if record {
        refuse_signing(verbs, &place, Signed::Commit)?;
        clean_tree(verbs, &place, &format!("a {what}"), COMMIT_OR_STASH)?;
    } else {
        unstaged_clean(verbs, &place, &format!("a {what} --no-commit"))?;
    }
    let commit = commit_named(verbs, &place, revision)?;
    verbs
        .ports()
        .git
        .pick(&place, &commit, revert, record)
        .map(MergeAnswer::from)
        .map_err(write_failed)
}

/// End the cherry-pick or revert in progress: `--quit` keeps the index and the files, `--abort` puts back what it
/// staged or left conflicted.
pub fn pick_end(verbs: &Verbs, revert: bool, abort: bool) -> Result<PickEndAnswer, OpError> {
    let what = match (revert, abort) {
        (false, false) => "cherry-pick --quit",
        (false, true) => "cherry-pick --abort",
        (true, false) => "revert --quit",
        (true, true) => "revert --abort",
    };
    let place = verbs.place_to_write(what)?;
    let ended = verbs
        .ports()
        .git
        .pick_end(&place, abort)
        .map_err(write_failed)?;
    Ok(PickEndAnswer { ended })
}

/// Refused when a file differs from the index or a path is conflicted: staged changes are allowed.
fn unstaged_clean(verbs: &Verbs, place: &GitPlace, what: &str) -> Result<(), OpError> {
    let status = verbs.ports().git.status(place).map_err(failed)?;
    let unstaged = status
        .entries
        .iter()
        .find(|e| e.kind == GitEntryKind::Unmerged || e.worktree.is_some());
    match unstaged {
        None => Ok(()),
        Some(entry) => Err(OpError::new(
            OpErrorKind::PreconditionFailed,
            super::DIRTY_TREE,
            format!(
                "{} has unstaged changes ({} among them): {what} would mix them in or lose them",
                printable(&place.toplevel.to_string_lossy()),
                printable(&entry.path)
            ),
        )
        .with_suggestion("stage, commit or stash the changes first")),
    }
}
