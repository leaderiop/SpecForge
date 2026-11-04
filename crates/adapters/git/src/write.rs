//! The `Git` port's writes (ADR 0339 D5, the S2 spike). A commit's object and its ref move are gix's; what meets the
//! index or the working tree is libgit2's (`git2`, built from source), because gix 0.84.0 has no worktree-to-index
//! pipeline (ignore rules under a directory, filters) and no checkout over an existing tree. Neither runs a program:
//! a hook is the caller's to run, through the Process port.

use std::path::Path;

use specforge_effects::git::{
    Committed, DiffQuery, DiffSide, Git as _, GitError, GitPlace, Sha, Staging,
};

use crate::{Gix, failed, id, open};

/// A write only in a repository this user owns: one another user owns is read with reduced trust, and git refuses to
/// change it.
fn owned(place: &GitPlace) -> Result<(), GitError> {
    match gix::sec::Trust::from_path_ownership(&place.git_dir) {
        Ok(gix::sec::Trust::Full) => Ok(()),
        _ => Err(GitError(format!(
            "{} belongs to another user: git changes only a repository its user owns",
            place.git_dir.display()
        ))),
    }
}

/// The operation in progress in this worktree that no write of the port continues or may end: a rebase, an `am`, a
/// bisect, or a sequencer (a cherry-pick or revert of several commits), as git's own state files say. A write in the
/// middle of one (a commit at a rebase's `edit` stop) would end it and lose its todo list.
fn foreign_operation(place: &GitPlace) -> Option<&'static str> {
    let at = |name: &str| place.git_dir.join(name).exists();
    if at("rebase-apply") {
        Some(if at("rebase-apply/applying") {
            "an am"
        } else {
            "a rebase"
        })
    } else if at("rebase-merge") {
        Some("a rebase")
    } else if at("BISECT_LOG") {
        Some("a bisect")
    } else if at("sequencer") {
        Some("a cherry-pick or revert of several commits")
    } else {
        None
    }
}

/// A write only where [`owned`] and with no [`foreign_operation`] in progress.
fn writable(place: &GitPlace) -> Result<(), GitError> {
    owned(place)?;
    match foreign_operation(place) {
        Some(operation) => Err(GitError(format!(
            "{operation} is in progress: finish or abort it with git first"
        ))),
        None => Ok(()),
    }
}

/// The repository at `place` through libgit2, once [`owned`], to read.
pub(crate) fn libgit2_read(place: &GitPlace) -> Result<git2::Repository, GitError> {
    owned(place)?;
    git2::Repository::open(&place.toplevel).map_err(|e| failed("open the repository", &e))
}

/// The repository at `place` through libgit2, once [`writable`].
pub(crate) fn libgit2(place: &GitPlace) -> Result<git2::Repository, GitError> {
    writable(place)?;
    libgit2_read(place)
}

/// The repository at `place` through gix, once [`writable`].
pub(crate) fn gix_owned(place: &GitPlace) -> Result<gix::Repository, GitError> {
    writable(place)?;
    open(place)
}

/// The files of a merge, cherry-pick or revert a commit concludes, which git removes once it is recorded. Never
/// libgit2's `cleanup_state`, which also deletes a rebase's, a bisect's and a sequencer's.
const CONCLUDED: [&str; 6] = [
    "MERGE_HEAD",
    "MERGE_MSG",
    "MERGE_MODE",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "AUTO_MERGE",
];

/// End the merge, cherry-pick or revert in progress in this worktree: its own files go, nothing else.
pub(crate) fn conclude(place: &GitPlace) -> Result<(), GitError> {
    for name in CONCLUDED {
        match std::fs::remove_file(place.git_dir.join(name)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(failed("end the operation the commit concludes", &e));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Where git hands a commit message to a hook: `COMMIT_EDITMSG` in this worktree's git directory.
fn message_file(place: &GitPlace) -> std::path::PathBuf {
    place.git_dir.join("COMMIT_EDITMSG")
}

pub(crate) fn write_commit_message(
    place: &GitPlace,
    message: &str,
) -> Result<std::path::PathBuf, GitError> {
    owned(place)?;
    let file = message_file(place);
    std::fs::write(&file, message).map_err(|e| failed("hand the message to its hook", &e))?;
    Ok(file)
}

pub(crate) fn read_commit_message(place: &GitPlace) -> Result<String, GitError> {
    std::fs::read_to_string(message_file(place))
        .map_err(|e| failed("read the message its hook left", &e))
}

pub(crate) fn stage(place: &GitPlace, staging: &Staging) -> Result<(), GitError> {
    let repo = libgit2(place)?;
    let mut index = repo.index().map_err(|e| failed("read the index", &e))?;
    let staged = |e: git2::Error| failed("stage the files", &e);
    if !staging.add.is_empty() {
        index
            .add_all(&staging.add, git2::IndexAddOption::DEFAULT, None)
            .map_err(staged)?;
        // a deleted file leaves the index, as `git add <path>` does
        index.update_all(&staging.add, None).map_err(staged)?;
    }
    if !staging.remove.is_empty() {
        index.remove_all(&staging.remove, None).map_err(staged)?;
    }
    if staging.all {
        index.update_all(["*"], None).map_err(staged)?;
    }
    index.write().map_err(|e| failed("write the index", &e))
}

pub(crate) fn commit(
    gix_port: &Gix,
    place: &GitPlace,
    message: &str,
    allow_empty: bool,
) -> Result<Committed, GitError> {
    // a merge the user resolved concludes as git concludes it: MERGE_HEAD's commits are parents too, and the
    // operation's state (a merge's, a cherry-pick's, a revert's) is ended once the commit is recorded
    let mut repo = libgit2(place)?;
    let state = repo.state();
    let parents = if state == git2::RepositoryState::Merge {
        let mut parents: Vec<Sha> = head_of(&repo).into_iter().collect();
        repo.mergehead_foreach(|oid| {
            parents.push(Sha::new(oid.to_string()));
            true
        })
        .map_err(|e| failed("read MERGE_HEAD", &e))?;
        Some(parents)
    } else {
        None
    };
    let merging = matches!(
        state,
        git2::RepositoryState::Merge
            | git2::RepositoryState::CherryPick
            | git2::RepositoryState::Revert
    );
    let committed = record(
        gix_port,
        place,
        message,
        parents,
        allow_empty || merging,
        None,
    )?;
    if merging {
        conclude(place)?;
    }
    Ok(committed)
}

/// `HEAD`'s commit, when it has one.
pub(crate) fn head_of(repo: &git2::Repository) -> Option<Sha> {
    repo.head()
        .ok()
        .and_then(|head| head.target())
        .map(|oid| Sha::new(oid.to_string()))
}

/// Who made a commit another one records again (a cherry-pick keeps its author), its time as git writes it
/// (`<seconds> <+hhmm>`).
pub(crate) struct Author {
    pub name: String,
    pub email: String,
    pub time: String,
}

/// Record the index as a commit on `HEAD`: its parents `HEAD`'s commit unless `parents` names them (a merge), its
/// author the configured identity unless `author` names one.
pub(crate) fn record(
    gix_port: &Gix,
    place: &GitPlace,
    message: &str,
    parents: Option<Vec<Sha>>,
    allow_empty: bool,
    author: Option<&Author>,
) -> Result<Committed, GitError> {
    let tree = {
        let repo = libgit2(place)?;
        let mut index = repo.index().map_err(|e| failed("read the index", &e))?;
        if index.has_conflicts() {
            return Err(GitError(
                "the index holds a conflict: resolve it and stage the files first".into(),
            ));
        }
        let tree = index
            .write_tree()
            .map_err(|e| failed("write the index's tree", &e))?;
        gix::ObjectId::from_hex(tree.to_string().as_bytes())
            .map_err(|e| failed("read the index's tree", &e))?
    };
    let repo = gix_owned(place)?;
    let parents: Vec<gix::ObjectId> = match parents {
        Some(named) => named.iter().map(id).collect::<Result<_, _>>()?,
        None => repo
            .head_id()
            .ok()
            .map(gix::Id::detach)
            .into_iter()
            .collect(),
    };
    let parent = parents.first().copied();
    let parent_tree = match parent {
        Some(parent) => repo
            .find_commit(parent)
            .map_err(|e| failed("read HEAD", &e))?
            .tree_id()
            .map_err(|e| failed("read HEAD's tree", &e))?
            .detach(),
        None => repo.empty_tree().id,
    };
    if !allow_empty && parent_tree == tree {
        return Err(GitError(
            "nothing to commit: the index matches HEAD (stage a change, or allow an empty commit)"
                .into(),
        ));
    }
    let branch = repo
        .head_name()
        .map_err(|e| failed("read HEAD", &e))?
        .map(|name| name.shorten().to_string());
    let recorded = match author {
        None => repo.commit("HEAD", message, tree, parents.clone()),
        Some(author) => {
            let committer = repo
                .committer()
                .ok_or_else(|| {
                    GitError(
                        "no identity is configured to commit as (user.name, user.email)".into(),
                    )
                })?
                .map_err(|e| failed("read the committer", &e))?;
            let author = gix::actor::SignatureRef {
                name: author.name.as_str().into(),
                email: author.email.as_str().into(),
                time: &author.time,
            };
            repo.commit_as(committer, author, "HEAD", message, tree, parents.clone())
        }
    };
    let sha = recorded
        .map_err(|e| failed("record the commit", &e))?
        .detach();
    let sha = Sha::new(sha.to_string());
    let files = match &parent {
        Some(parent) => gix_port
            .diff(
                place,
                &DiffQuery {
                    base: Sha::new(parent.to_string()),
                    head: DiffSide::Commit(sha.clone()),
                    paths: Vec::new(),
                    patch: false,
                },
            )?
            .into_iter()
            .map(|file| file.path)
            .collect(),
        None => gix_port
            .tree(place, &sha, Path::new(""))?
            .into_iter()
            .map(|(path, _)| path.to_string_lossy().replace('\\', "/"))
            .collect(),
    };
    Ok(Committed {
        sha,
        branch,
        parents: parents.iter().map(|p| Sha::new(p.to_string())).collect(),
        files,
    })
}
