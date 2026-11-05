//! The `Git` port's merge, reset, restore, stash, cherry-pick and revert (ADR 0339 D5). A ref move is
//! gix's (a fast-forward, `reset --soft`); what writes the index and the files is libgit2's, always with a safe
//! checkout, so a file the worktree changed is never overwritten. A commit the operation makes is recorded as
//! `git commit` records one.

use gix::refs::transaction::PreviousValue;
use specforge_effects::git::{GitConflict, GitError, GitPlace, GitStash, Merged, ResetMode, Sha};

use crate::write::{Author, conclude, gix_owned, head_of, libgit2, libgit2_read, record, safe};
use crate::{Gix, failed, id};

fn oid(sha: &Sha) -> Result<git2::Oid, GitError> {
    git2::Oid::from_str(sha.as_str()).map_err(|e| failed("read a commit id", &e))
}

/// Move `HEAD`'s branch, or a detached `HEAD` itself, from `from` to `to` (gix: a ref move), once `between` ran:
/// the ref is locked and checked to still hold `from` first, so no other git moves it meanwhile, and a failure of
/// `between` leaves it as it was.
fn move_head(
    place: &GitPlace,
    from: Option<&Sha>,
    to: &Sha,
    log: &str,
    between: impl FnOnce() -> Result<(), GitError>,
) -> Result<(), GitError> {
    use gix::refs::transaction::{Change, LogChange, RefEdit, RefLog};
    let repo = gix_owned(place)?;
    let name = repo
        .head_name()
        .map_err(|e| failed("read HEAD", &e))?
        .map_or_else(|| "HEAD".to_string(), |name| name.to_string());
    let expected = match from {
        Some(from) => PreviousValue::MustExistAndMatch(gix::refs::Target::Object(id(from)?)),
        None => PreviousValue::MustNotExist,
    };
    let edit = RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: false,
                message: log.into(),
            },
            expected,
            new: gix::refs::Target::Object(id(to)?),
        },
        name: name
            .as_str()
            .try_into()
            .map_err(|e| failed("name HEAD's ref", &e))?,
        deref: false,
    };
    let fail = gix::lock::acquire::Fail::Immediately;
    let locked = repo
        .refs
        .transaction()
        .prepare([edit], fail, fail)
        .map_err(|e| failed("lock HEAD's ref", &e))?;
    between()?;
    let committer = repo
        .committer()
        .transpose()
        .map_err(|e| failed("read the committer", &e))?;
    locked
        .commit(committer)
        .map_err(|e| failed("move HEAD", &e))?;
    Ok(())
}

/// The index's conflicts, by path, as `git status` names each.
fn conflicts(repo: &git2::Repository) -> Result<Vec<GitConflict>, GitError> {
    let index = repo.index().map_err(|e| failed("read the index", &e))?;
    let mut out = Vec::new();
    for conflict in index
        .conflicts()
        .map_err(|e| failed("read the index", &e))?
    {
        let conflict = conflict.map_err(|e| failed("read the index", &e))?;
        let path = [&conflict.our, &conflict.their, &conflict.ancestor]
            .into_iter()
            .flatten()
            .next()
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
            .unwrap_or_default();
        let kind = match (&conflict.ancestor, &conflict.our, &conflict.their) {
            (None, Some(_), Some(_)) => "both_added",
            (Some(_), None, Some(_)) => "deleted_by_us",
            (Some(_), Some(_), None) => "deleted_by_them",
            _ => "both_modified",
        };
        out.push(GitConflict {
            path,
            kind: kind.to_string(),
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

pub(crate) fn merge(
    port: &Gix,
    place: &GitPlace,
    theirs: &Sha,
    fast_forward: Option<bool>,
    message: &str,
) -> Result<Merged, GitError> {
    let repo = libgit2(place)?;
    let annotated = repo
        .find_annotated_commit(oid(theirs)?)
        .map_err(|e| failed("read the commit to merge", &e))?;
    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|e| failed("compare the histories", &e))?;
    let head = head_of(&repo);
    if analysis.is_up_to_date() {
        return Ok(Merged {
            sha: head,
            fast_forward: false,
            conflicts: Vec::new(),
        });
    }
    if analysis.is_fast_forward() && fast_forward != Some(false) {
        let target = repo
            .find_object(oid(theirs)?, None)
            .map_err(|e| failed("read the commit to merge", &e))?;
        move_head(place, head.as_ref(), theirs, "merge: Fast-forward", || {
            repo.checkout_tree(&target, Some(&mut safe()))
                .map_err(|e| failed("check out the merged files", &e))
        })?;
        return Ok(Merged {
            sha: Some(theirs.clone()),
            fast_forward: true,
            conflicts: Vec::new(),
        });
    }
    if fast_forward == Some(true) {
        return Err(GitError(
            "the histories have diverged: no fast-forward is possible".into(),
        ));
    }
    let Some(ours) = head else {
        return Err(GitError("HEAD has no commit to merge into".into()));
    };
    repo.merge(&[&annotated], None, Some(&mut safe()))
        .map_err(|e| failed("merge", &e))?;
    let conflicts = conflicts(&repo)?;
    if !conflicts.is_empty() {
        // left as git leaves it: the merge in progress, for the user to resolve and commit
        return Ok(Merged {
            sha: None,
            fast_forward: false,
            conflicts,
        });
    }
    let committed = record(
        port,
        place,
        message,
        Some(vec![ours, theirs.clone()]),
        true,
        None,
    )?;
    conclude(place)?;
    Ok(Merged {
        sha: Some(committed.sha),
        fast_forward: false,
        conflicts: Vec::new(),
    })
}

pub(crate) fn reset(place: &GitPlace, to: &Sha, mode: ResetMode) -> Result<(), GitError> {
    if mode == ResetMode::Soft {
        // as git: "Cannot do a soft reset in the middle of a merge"
        if place.git_dir.join("MERGE_HEAD").exists() {
            return Err(GitError(
                "a merge is in progress: a soft reset would keep its index, so conclude or abort it first".into(),
            ));
        }
        let head = head_of(&libgit2(place)?);
        return move_head(place, head.as_ref(), to, "reset: moving", || Ok(()));
    }
    let repo = libgit2(place)?;
    let target = repo
        .find_object(oid(to)?, None)
        .map_err(|e| failed("read the commit to reset to", &e))?;
    let kind = if mode == ResetMode::Hard {
        git2::ResetType::Hard
    } else {
        git2::ResetType::Mixed
    };
    repo.reset(&target, kind, None)
        .map_err(|e| failed("reset", &e))
}

pub(crate) fn restore(
    place: &GitPlace,
    paths: &[String],
    source: Option<&Sha>,
    staged: bool,
) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let from = match source {
        Some(sha) => Some(
            repo.find_object(oid(sha)?, None)
                .map_err(|e| failed("read the commit to restore from", &e))?,
        ),
        None => None,
    };
    if staged {
        let head = match from {
            Some(from) => Some(from),
            None => repo
                .head()
                .and_then(|head| head.peel(git2::ObjectType::Commit))
                .ok(),
        };
        return repo
            .reset_default(head.as_ref(), paths)
            .map_err(|e| failed("restore the index", &e));
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    // the user asked for these files back: their changes are what is discarded
    checkout.force().remove_untracked(false);
    for path in paths {
        checkout.path(path);
    }
    match from {
        Some(from) => repo.checkout_tree(&from, Some(&mut checkout)),
        None => repo.checkout_index(None, Some(&mut checkout)),
    }
    .map_err(|e| failed("restore the files", &e))
}

pub(crate) fn stash_push(
    place: &GitPlace,
    message: Option<&str>,
    untracked: bool,
) -> Result<Sha, GitError> {
    let mut repo = libgit2(place)?;
    let stasher = repo
        .signature()
        .map_err(|e| failed("name who stashes (user.name, user.email)", &e))?;
    let flags = untracked.then_some(git2::StashFlags::INCLUDE_UNTRACKED);
    repo.stash_save2(&stasher, message, flags)
        .map(|oid| Sha::new(oid.to_string()))
        .map_err(|e| failed("stash the changes", &e))
}

pub(crate) fn stash_pop(place: &GitPlace, index: usize) -> Result<(), GitError> {
    let mut repo = libgit2(place)?;
    let mut options = git2::StashApplyOptions::new();
    options.checkout_options(safe());
    repo.stash_pop(index, Some(&mut options))
        .map_err(|e| failed(&format!("apply stash@{{{index}}}"), &e))
}

pub(crate) fn stashes(place: &GitPlace) -> Result<Vec<GitStash>, GitError> {
    let mut repo = libgit2_read(place)?;
    let mut out = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        out.push(GitStash {
            index,
            message: message.to_string(),
            sha: Sha::new(oid.to_string()),
        });
        true
    })
    .map_err(|e| failed("list the stashes", &e))?;
    Ok(out)
}

/// Apply `commit` onto `HEAD` (or its inverse, `revert`), and record it when nothing conflicts: a cherry-pick keeps the
/// commit's author and message, a revert says what it reverts.
pub(crate) fn pick(
    port: &Gix,
    place: &GitPlace,
    commit: &Sha,
    revert: bool,
) -> Result<Merged, GitError> {
    let repo = libgit2(place)?;
    let Some(head) = head_of(&repo) else {
        return Err(GitError("HEAD has no commit to apply onto".into()));
    };
    let picked = repo
        .find_commit(oid(commit)?)
        .map_err(|e| failed("read the commit", &e))?;
    let applied = if revert {
        repo.revert(&picked, None)
    } else {
        repo.cherrypick(&picked, None)
    };
    applied.map_err(|e| failed(if revert { "revert" } else { "cherry-pick" }, &e))?;
    let conflicts = conflicts(&repo)?;
    if !conflicts.is_empty() {
        return Ok(Merged {
            sha: None,
            fast_forward: false,
            conflicts,
        });
    }
    let original = String::from_utf8_lossy(picked.message_bytes()).into_owned();
    let (message, author) = if revert {
        let subject = original.lines().next().unwrap_or_default();
        (
            format!(
                "Revert \"{subject}\"\n\nThis reverts commit {commit}.\n",
                commit = commit.as_str()
            ),
            None,
        )
    } else {
        let who = picked.author();
        let offset = who.when().offset_minutes().saturating_mul(60);
        // `write_to`, not `Display`: an offset past ±99 hours (a hostile commit's) is an error, never a panic.
        let mut time = Vec::new();
        gix::date::Time::new(who.when().seconds(), offset)
            .write_to(&mut time)
            .map_err(|e| failed("write the picked commit's author time", &e))?;
        let author = Author {
            name: String::from_utf8_lossy(who.name_bytes()).into_owned(),
            email: String::from_utf8_lossy(who.email_bytes()).into_owned(),
            time: String::from_utf8_lossy(&time).into_owned(),
        };
        (original, Some(author))
    };
    let committed = record(
        port,
        place,
        &message,
        Some(vec![head]),
        false,
        author.as_ref(),
    )?;
    conclude(place)?;
    Ok(Merged {
        sha: Some(committed.sha),
        fast_forward: false,
        conflicts: Vec::new(),
    })
}
