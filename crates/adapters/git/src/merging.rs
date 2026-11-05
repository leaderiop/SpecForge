//! The `Git` port's merge, reset, restore, stash, cherry-pick and revert (ADR 0339 D5). A ref move is
//! gix's (a fast-forward, `reset --soft`); what writes the index and the files is libgit2's, always with a safe
//! checkout, so a file the worktree changed is never overwritten. A commit the operation makes is recorded as
//! `git commit` records one.

use gix::refs::transaction::PreviousValue;
use specforge_effects::git::{
    ConflictSide, GitConflict, GitError, GitPlace, GitStash, Merged, Merging, ResetMode, Restoring,
    Sha,
};

use crate::write::{author_of, conclude, gix_owned, head_of, libgit2, libgit2_read, record, safe};
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
    merging: &Merging,
) -> Result<Merged, GitError> {
    let fast_forward = merging.fast_forward;
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
    if analysis.is_fast_forward() && fast_forward != Some(false) && !merging.squash {
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
    if fast_forward == Some(true) && !analysis.is_fast_forward() {
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
    if merging.squash {
        squashed(place, &repo, &ours, theirs)?;
    }
    if merging.squash || merging.no_commit {
        // left staged, as git leaves it, for the commit that records it
        return Ok(Merged {
            sha: None,
            fast_forward: false,
            conflicts,
        });
    }
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
        &merging.message,
        Some(vec![ours, theirs.clone()]),
        true,
        None,
        &[],
    )?;
    conclude(place)?;
    Ok(Merged {
        sha: Some(committed.sha),
        fast_forward: false,
        conflicts: Vec::new(),
    })
}

/// A merge made a squash, as git leaves one: no `MERGE_HEAD` nor `MERGE_MODE`, and `SQUASH_MSG` naming each commit
/// it brings, newest first, in place of `MERGE_MSG`.
fn squashed(
    place: &GitPlace,
    repo: &git2::Repository,
    ours: &Sha,
    theirs: &Sha,
) -> Result<(), GitError> {
    let at = |e: git2::Error| failed("list the squashed commits", &e);
    let mut walk = repo.revwalk().map_err(at)?;
    walk.push(oid(theirs)?).map_err(at)?;
    walk.hide(oid(ours)?).map_err(at)?;
    let mut message = String::from("Squashed commit of the following:\n");
    for commit in walk {
        let commit = repo.find_commit(commit.map_err(at)?).map_err(at)?;
        let who = commit.author();
        message.push_str(&format!(
            "\ncommit {}\nAuthor: {} <{}>\n\n",
            commit.id(),
            String::from_utf8_lossy(who.name_bytes()),
            String::from_utf8_lossy(who.email_bytes())
        ));
        for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
            message.push_str(&format!("    {line}\n"));
        }
    }
    for name in ["MERGE_HEAD", "MERGE_MODE", "MERGE_MSG"] {
        match std::fs::remove_file(place.git_dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(failed(&format!("remove {name}"), &e));
            }
            _ => {}
        }
    }
    std::fs::write(place.git_dir.join("SQUASH_MSG"), message)
        .map_err(|e| failed("write SQUASH_MSG", &e))
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
    restoring: &Restoring,
) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    // libgit2 knows no pathspec magic: gix matches it against the index and the source's files, named one by one
    let magic = paths.iter().any(|path| path.starts_with(':'));
    let expanded;
    let paths = if magic {
        expanded = matched(&repo, paths, restoring.source.as_ref())?;
        expanded.as_slice()
    } else {
        paths
    };
    let mut checkout = git2::build::CheckoutBuilder::new();
    // the user asked for these files back: their changes are what is discarded
    checkout
        .force()
        .remove_untracked(false)
        .disable_pathspec_match(magic);
    for path in paths {
        checkout.path(path);
    }
    if let Some(side) = restoring.side {
        match side {
            ConflictSide::Ours => checkout.use_ours(true),
            ConflictSide::Theirs => checkout.use_theirs(true),
        };
        return repo
            .checkout_index(None, Some(&mut checkout))
            .map_err(|e| failed("restore the files from a side of their conflict", &e));
    }
    let from = match &restoring.source {
        Some(sha) => Some(
            repo.find_object(oid(sha)?, None)
                .map_err(|e| failed("read the commit to restore from", &e))?,
        ),
        None => None,
    };
    if restoring.staged {
        let head = match from {
            Some(from) => Some(from),
            None => repo
                .head()
                .and_then(|head| head.peel(git2::ObjectType::Commit))
                .ok(),
        };
        if restoring.worktree {
            // a checkout of the source's tree writes the index's entries and the files both
            let Some(head) = head else {
                return Err(GitError("HEAD has no commit to restore from".into()));
            };
            return repo
                .checkout_tree(&head, Some(&mut checkout))
                .map_err(|e| failed("restore the index and the files", &e));
        }
        return repo
            .reset_default(head.as_ref(), paths)
            .map_err(|e| failed("restore the index", &e));
    }
    match from {
        Some(from) => repo.checkout_tree(&from, Some(&mut checkout)),
        None => repo.checkout_index(None, Some(&mut checkout)),
    }
    .map_err(|e| failed("restore the files", &e))
}

/// The files of the index and of `source` (else `HEAD`) that `specs` name, each spec matching one at least.
fn matched(
    repo: &git2::Repository,
    specs: &[String],
    source: Option<&Sha>,
) -> Result<Vec<String>, GitError> {
    let mut files = std::collections::BTreeSet::new();
    let index = repo.index().map_err(|e| failed("read the index", &e))?;
    files.extend(
        index
            .iter()
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned()),
    );
    let commit = match source {
        Some(sha) => repo.find_commit(oid(sha)?).ok(),
        None => repo.head().and_then(|head| head.peel_to_commit()).ok(),
    };
    if let Some(tree) = commit.and_then(|commit| commit.tree().ok()) {
        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                files.insert(format!(
                    "{dir}{}",
                    String::from_utf8_lossy(entry.name_bytes())
                ));
            }
            git2::TreeWalkResult::Ok
        })
        .map_err(|e| failed("read the files to restore from", &e))?;
    }
    let mut named = Vec::new();
    for spec in specs {
        let mut search = crate::write::pathspec(spec)?;
        let found: Vec<&String> = files.iter().filter(|file| search.matches(file)).collect();
        if found.is_empty() {
            return Err(GitError(format!("{spec} did not match any files")));
        }
        named.extend(found.into_iter().cloned());
    }
    named.sort();
    named.dedup();
    Ok(named)
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
        (reverting(&original, commit), None)
    } else {
        (original, Some(author_of(&picked)?))
    };
    let committed = record(
        port,
        place,
        &message,
        Some(vec![head]),
        false,
        author.as_ref(),
        &[],
    )?;
    conclude(place)?;
    Ok(Merged {
        sha: Some(committed.sha),
        fast_forward: false,
        conflicts: Vec::new(),
    })
}

/// The message git gives a revert of the commit `original` is the message of.
fn reverting(original: &str, commit: &Sha) -> String {
    let subject = original.lines().next().unwrap_or_default();
    format!(
        "Revert \"{subject}\"\n\nThis reverts commit {commit}.\n",
        commit = commit.as_str()
    )
}

/// Apply `commit` (or its inverse, `revert`) onto the index as it stands and leave it staged, the files with it
/// (`git cherry-pick -n`, `git revert -n`): a three-way merge of trees, so a second one stacks on the first. As git,
/// a revert leaves `REVERT_HEAD` and a cherry-pick no `CHERRY_PICK_HEAD`; both leave the message in `MERGE_MSG`.
pub(crate) fn pick_staged(
    place: &GitPlace,
    commit: &Sha,
    revert: bool,
) -> Result<Merged, GitError> {
    let repo = libgit2(place)?;
    let read = |e: git2::Error| failed("read the commit", &e);
    let picked = repo.find_commit(oid(commit)?).map_err(read)?;
    if picked.parent_count() > 1 {
        return Err(GitError(format!(
            "{} is a merge: git needs -m to say which parent it undoes or applies, which specforge does not take",
            commit.short()
        )));
    }
    let parent = match picked.parent(0) {
        Ok(parent) => parent.tree().map_err(read)?,
        // a root commit: against the empty tree, as git takes it
        Err(_) => {
            let empty = repo
                .treebuilder(None)
                .and_then(|builder| builder.write())
                .map_err(|e| failed("write the empty tree", &e))?;
            repo.find_tree(empty).map_err(read)?
        }
    };
    let mut index = repo.index().map_err(|e| failed("read the index", &e))?;
    if index.has_conflicts() {
        return Err(GitError(
            "the index holds a conflict: resolve or abort it first".into(),
        ));
    }
    let ours = index
        .write_tree()
        .and_then(|tree| repo.find_tree(tree))
        .map_err(|e| failed("read the index as a tree", &e))?;
    let own = picked.tree().map_err(read)?;
    let (base, theirs) = if revert {
        (&own, &parent)
    } else {
        (&parent, &own)
    };
    let mut merged = repo
        .merge_trees(base, &ours, theirs, None)
        .map_err(|e| failed(if revert { "revert" } else { "cherry-pick" }, &e))?;
    let mut checkout = safe();
    checkout.allow_conflicts(true).conflict_style_merge(true);
    repo.checkout_index(Some(&mut merged), Some(&mut checkout))
        .map_err(|e| failed("check out the applied files", &e))?;
    let original = String::from_utf8_lossy(picked.message_bytes()).into_owned();
    let message = if revert {
        reverting(&original, commit)
    } else {
        original
    };
    let state = |name: &str, text: &str| {
        std::fs::write(place.git_dir.join(name), text)
            .map_err(|e| failed(&format!("write {name}"), &e))
    };
    state("MERGE_MSG", &message)?;
    if revert {
        state("REVERT_HEAD", &format!("{}\n", commit.as_str()))?;
    }
    Ok(Merged {
        sha: None,
        fast_forward: false,
        conflicts: conflicts(&repo)?,
    })
}

/// End the cherry-pick or revert in progress (`--quit`), and with `abort` put back as `HEAD` has them the paths
/// the index changed or holds conflicted, as git's `reset --merge` does, other files' changes kept (`--abort`).
pub(crate) fn pick_end(place: &GitPlace, abort: bool) -> Result<bool, GitError> {
    let repo = libgit2(place)?;
    let sequencer = place.git_dir.join("sequencer");
    let sequenced = std::fs::symlink_metadata(&sequencer).is_ok();
    let in_progress = sequenced
        || ["CHERRY_PICK_HEAD", "REVERT_HEAD"]
            .iter()
            .any(|name| place.git_dir.join(name).exists());
    if abort {
        if !in_progress {
            return Err(GitError("no cherry-pick or revert is in progress".into()));
        }
        if sequenced {
            return Err(GitError(
                "a sequence of picks git started is in progress: specforge aborts a single cherry-pick or revert: \
                 end it with specforge git cherry-pick --quit or specforge git revert --quit, then reset"
                    .into(),
            ));
        }
        rolled_back(&repo)?;
    }
    if sequenced {
        // a link is removed itself, never followed
        let gone = match std::fs::symlink_metadata(&sequencer) {
            Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(&sequencer),
            _ => std::fs::remove_file(&sequencer),
        };
        gone.map_err(|e| failed("remove the sequencer's state", &e))?;
    }
    conclude(place)?;
    Ok(in_progress)
}

/// The paths the index stages against `HEAD` or holds conflicted, back as `HEAD` has them in the index and on disk.
fn rolled_back(repo: &git2::Repository) -> Result<(), GitError> {
    let head = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|e| failed("read HEAD", &e))?;
    let tree = head.tree().map_err(|e| failed("read HEAD", &e))?;
    let index = repo.index().map_err(|e| failed("read the index", &e))?;
    let mut paths = std::collections::BTreeSet::new();
    for conflict in index
        .conflicts()
        .map_err(|e| failed("read the index", &e))?
    {
        let conflict = conflict.map_err(|e| failed("read the index", &e))?;
        for entry in [conflict.our, conflict.their, conflict.ancestor]
            .into_iter()
            .flatten()
        {
            paths.insert(String::from_utf8_lossy(&entry.path).into_owned());
        }
    }
    let staged = repo
        .diff_tree_to_index(Some(&tree), Some(&index), None)
        .map_err(|e| failed("compare the index with HEAD", &e))?;
    for delta in staged.deltas() {
        for file in [delta.old_file(), delta.new_file()] {
            if let Some(path) = file.path() {
                paths.insert(path.to_string_lossy().into_owned());
            }
        }
    }
    if paths.is_empty() {
        return Ok(());
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout
        .force()
        .remove_untracked(false)
        .disable_pathspec_match(true);
    for path in &paths {
        checkout.path(path);
    }
    repo.checkout_tree(tree.as_object(), Some(&mut checkout))
        .map_err(|e| failed("put the files back", &e))?;
    repo.reset_default(Some(head.as_object()), &paths)
        .map_err(|e| failed("put the index back", &e))
}
