//! The `Git` port's branch, tag, switch and worktree writes (ADR 0339 D5): a ref is gix's to write, a
//! checkout and a worktree libgit2's.

use std::path::{Path, PathBuf};

use gix::refs::transaction::PreviousValue;
use specforge_effects::git::{GitError, GitPlace, Sha};

use crate::write::{gix_owned, libgit2, safe};
use crate::{failed, id};

/// `refs/heads/<name>`, refused when it is no valid branch name.
fn branch_ref(name: &str) -> Result<gix::refs::FullName, GitError> {
    gix::refs::FullName::try_from(format!("refs/heads/{name}"))
        .map_err(|e| failed(&format!("name a branch {name:?}"), &e))
}

pub(crate) fn branch_create(place: &GitPlace, name: &str, at: &Sha) -> Result<(), GitError> {
    let repo = gix_owned(place)?;
    let full = branch_ref(name)?;
    // as for a tag: an existing branch is refused even when it names `at` already
    if repo
        .try_find_reference(&full)
        .map_err(|e| failed("read the branches", &e))?
        .is_some()
    {
        return Err(GitError(format!("the branch {name} already exists")));
    }
    repo.reference(
        full,
        id(at)?,
        PreviousValue::MustNotExist,
        format!("branch: Created from {}", at.short()),
    )
    .map_err(|e| failed(&format!("create the branch {name}"), &e))?;
    Ok(())
}

pub(crate) fn branch_delete(place: &GitPlace, name: &str) -> Result<Sha, GitError> {
    let repo = gix_owned(place)?;
    // as git, even with `--force`: a branch a worktree (the main one or a linked one) has checked out stays
    if let Some(there) = crate::refs::worktrees(&repo)?
        .into_iter()
        .find(|worktree| worktree.branch.as_deref() == Some(name))
    {
        return Err(GitError(format!(
            "the branch {name} is checked out in the worktree {}",
            there.path.display()
        )));
    }
    let reference = repo
        .find_reference(&branch_ref(name)?)
        .map_err(|e| failed(&format!("find the branch {name}"), &e))?;
    let sha = reference
        .target()
        .try_id()
        .map(|id| Sha::new(id.to_string()))
        .ok_or_else(|| GitError(format!("the branch {name} names another ref")))?;
    reference
        .delete()
        .map_err(|e| failed(&format!("delete the branch {name}"), &e))?;
    Ok(sha)
}

pub(crate) fn tag(
    place: &GitPlace,
    name: &str,
    at: &Sha,
    message: Option<&str>,
) -> Result<Sha, GitError> {
    let repo = gix_owned(place)?;
    let full = format!("refs/tags/{name}");
    let named = |e: &dyn std::fmt::Display| failed(&format!("create the tag {name}"), e);
    // gix takes an existing ref that already names the commit as written: git refuses any existing tag
    if repo
        .try_find_reference(full.as_str())
        .map_err(|e| named(&e))?
        .is_some()
    {
        return Err(GitError(format!("the tag {name} already exists")));
    }
    let tagged = match message {
        Some(message) => {
            let tagger = repo
                .committer()
                .ok_or_else(|| {
                    GitError("no identity is configured to tag as (user.name, user.email)".into())
                })?
                .map_err(|e| named(&e))?;
            repo.tag(
                name,
                id(at)?,
                gix::objs::Kind::Commit,
                Some(tagger),
                message,
                PreviousValue::MustNotExist,
            )
            .map_err(|e| named(&e))?
            .id()
            .detach()
        }
        None => repo
            .reference(full.as_str(), id(at)?, PreviousValue::MustNotExist, "tag")
            .map_err(|e| named(&e))?
            .id()
            .detach(),
    };
    Ok(Sha::new(tagged.to_string()))
}

pub(crate) fn switch(place: &GitPlace, branch: &str) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let full = branch_ref(branch)?.to_string();
    let target = repo
        .revparse_single(&full)
        .and_then(|object| object.peel(git2::ObjectType::Commit))
        .map_err(|e| failed(&format!("find the branch {branch}"), &e))?;
    repo.checkout_tree(&target, Some(&mut safe()))
        .map_err(|e| failed(&format!("switch to {branch}"), &e))?;
    repo.set_head(&full)
        .map_err(|e| failed(&format!("move HEAD to {branch}"), &e))
}

pub(crate) fn worktree_add(place: &GitPlace, path: &Path, branch: &str) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| GitError(format!("{} names no directory", path.display())))?;
    let reference = repo
        .find_reference(&branch_ref(branch)?.to_string())
        .map_err(|e| failed(&format!("find the branch {branch}"), &e))?;
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&reference));
    repo.worktree(name, path, Some(&options))
        .map_err(|e| failed(&format!("add a worktree at {}", path.display()), &e))?;
    Ok(())
}

/// The linked worktree whose top is `path`.
fn linked(repo: &git2::Repository, path: &Path) -> Result<git2::Worktree, GitError> {
    let names = repo
        .worktrees()
        .map_err(|e| failed("list the worktrees", &e))?;
    let wanted = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    for name in names.iter().flatten().flatten() {
        let Ok(worktree) = repo.find_worktree(name) else {
            continue;
        };
        let at = std::fs::canonicalize(worktree.path())
            .unwrap_or_else(|_| worktree.path().to_path_buf());
        if at == wanted {
            return Ok(worktree);
        }
    }
    Err(GitError(format!(
        "{} is no linked worktree of this repository",
        path.display()
    )))
}

pub(crate) fn worktree_remove(place: &GitPlace, path: &Path) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let worktree = linked(&repo, path)?;
    let mut options = git2::WorktreePruneOptions::new();
    options.valid(true).working_tree(true);
    worktree
        .prune(Some(&mut options))
        .map_err(|e| failed(&format!("remove the worktree at {}", path.display()), &e))
}

/// `git init`: `dir` made a repository by gix, `HEAD` on `main` whatever a config's `init.defaultBranch` says, unless
/// it holds one already.
pub(crate) fn init(dir: &Path) -> Result<bool, GitError> {
    if std::fs::symlink_metadata(dir.join(".git")).is_ok() {
        return Ok(false);
    }
    // isolated: no system or global config, whose init.defaultBranch (Git for Windows sets master) gix would follow;
    // the branch is gix's own default, main
    gix::ThreadSafeRepository::init_opts(
        dir,
        gix::create::Kind::WithWorktree,
        gix::create::Options::default(),
        gix::open::Options::isolated(),
    )
    .map_err(|e| failed(&format!("make {} a repository", dir.display()), &e))?;
    Ok(true)
}

pub(crate) fn worktree_lock(
    place: &GitPlace,
    path: &Path,
    lock: bool,
    reason: Option<&str>,
) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let worktree = linked(&repo, path)?;
    let at = |e: git2::Error| failed(&format!("read the worktree at {}", path.display()), &e);
    let locked = !matches!(
        worktree.is_locked().map_err(at)?,
        git2::WorktreeLockStatus::Unlocked
    );
    match (lock, locked) {
        (true, true) => Err(GitError(format!("{} is already locked", path.display()))),
        (false, false) => Err(GitError(format!("{} is not locked", path.display()))),
        (true, false) => worktree.lock(reason).map_err(at),
        (false, true) => worktree.unlock().map_err(at),
    }
}

pub(crate) fn worktree_prune(place: &GitPlace) -> Result<Vec<PathBuf>, GitError> {
    let repo = libgit2(place)?;
    let names = repo
        .worktrees()
        .map_err(|e| failed("list the worktrees", &e))?;
    let mut pruned = Vec::new();
    for name in names.iter().flatten().flatten() {
        let Ok(worktree) = repo.find_worktree(name) else {
            continue;
        };
        // prunable: its directory is gone and it is not locked
        if worktree.is_prunable(None).unwrap_or(false) {
            let at = worktree.path().to_path_buf();
            worktree
                .prune(None)
                .map_err(|e| failed(&format!("prune the worktree {name}"), &e))?;
            pruned.push(at);
        }
    }
    Ok(pruned)
}
