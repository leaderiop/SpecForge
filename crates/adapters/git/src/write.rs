//! The `Git` port's writes (ADR 0339 D5). A commit's object and its ref move are gix's; what meets the
//! index or the working tree is libgit2's (`git2`, built from source), because gix 0.84.0 has no worktree-to-index
//! pipeline (ignore rules under a directory, filters) and no checkout over an existing tree. Neither runs a program:
//! a hook is the caller's to run, through the Process port.

use std::path::Path;

use specforge_effects::git::{
    Committed, DiffQuery, DiffSide, Git as _, GitError, GitPlace, Recording, Removal, Sha, Staging,
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
pub(crate) fn foreign_operation(place: &GitPlace) -> Option<&'static str> {
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
        Some(SEQUENCER)
    } else {
        None
    }
}

/// A sequencer's [`foreign_operation`], which `cherry-pick --quit` and `revert --quit` end.
pub(crate) const SEQUENCER: &str = "a cherry-pick or revert of several commits";

/// A write only where [`owned`] and with no [`foreign_operation`] in progress.
pub(crate) fn writable(place: &GitPlace) -> Result<(), GitError> {
    owned(place)?;
    match foreign_operation(place) {
        Some(SEQUENCER) => Err(GitError(format!(
            "{SEQUENCER} is in progress: finish it with git, or end it with specforge git cherry-pick --quit \
             (specforge git revert --quit) first"
        ))),
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

/// The files of a merge, squash, cherry-pick or revert a commit concludes, which git removes once it is recorded. Never
/// libgit2's `cleanup_state`, which also deletes a rebase's, a bisect's and a sequencer's.
const CONCLUDED: [&str; 7] = [
    "MERGE_HEAD",
    "MERGE_MSG",
    "MERGE_MODE",
    "SQUASH_MSG",
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
    for path in staging.add.iter().filter(|path| *path != "*") {
        added(place, &repo, &index, path, staging.force)?;
    }
    // Each file staged from the worktree is asked its `filter` attribute first: the first that names a driver
    // stops the staging (libgit2 runs none, so it would be staged raw), and the refusal names it.
    let refused = std::cell::RefCell::new(None);
    let mut unfiltered = |path: &Path, _: &[u8]| -> i32 {
        // a positive answer skips the file, as libgit2 reads it
        if staging
            .except
            .iter()
            .any(|except| except == "*" || path.starts_with(except))
        {
            return 1;
        }
        // a deletion reads no file, so no filter runs on it
        if !place.toplevel.join(path).exists() {
            return 0;
        }
        match filter_driver(&repo, path) {
            Ok(None) => 0,
            Ok(Some(driver)) => {
                *refused.borrow_mut() = Some(GitError(format!(
                    "{} names the filter driver `{}` in its attributes, which specforge does not run: stage it with \
                     git",
                    path.display(),
                    driver
                )));
                -1
            }
            Err(error) => {
                *refused.borrow_mut() = Some(error);
                -1
            }
        }
    };
    let staged = |e: git2::Error| {
        refused
            .borrow_mut()
            .take()
            .unwrap_or_else(|| failed("stage the files", &e))
    };
    if !staging.add.is_empty() {
        let option = if staging.force {
            git2::IndexAddOption::FORCE
        } else {
            git2::IndexAddOption::DEFAULT
        };
        index
            .add_all(&staging.add, option, Some(&mut unfiltered))
            .map_err(staged)?;
        // a deleted file leaves the index, as `git add <path>` does
        index
            .update_all(&staging.add, Some(&mut unfiltered))
            .map_err(staged)?;
    }
    if !staging.remove.is_empty() {
        index.remove_all(&staging.remove, None).map_err(staged)?;
    }
    if staging.all {
        index
            .update_all(["*"], Some(&mut unfiltered))
            .map_err(staged)?;
    }
    index.write().map_err(|e| failed("write the index", &e))
}

/// `git rm`: the entries at `paths` out of the index and, unless cached, their files off the disk; nothing changes
/// when a path is refused. A file is removed only below real directories of the worktree, never through a link.
pub(crate) fn remove(
    place: &GitPlace,
    paths: &[String],
    removal: Removal,
) -> Result<Vec<String>, GitError> {
    let repo = libgit2(place)?;
    let mut index = repo.index().map_err(|e| failed("read the index", &e))?;
    let files: Vec<String> = index
        .iter()
        .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
        .collect();
    let mut specs = pathspecs(paths)?;
    if let Some(spec) = specs.unmatched(&files)
        && !removal.ignore_unmatch
    {
        return Err(GitError(format!("{spec} did not match any files")));
    }
    let mut removed = std::collections::BTreeSet::new();
    for file in files {
        let Some(named) = specs.named(&file) else {
            continue;
        };
        if named.whole && !removal.recursive {
            return Err(GitError(format!(
                "not removing {} recursively without -r",
                named.spec
            )));
        }
        removed.insert(file);
    }
    if !removal.force {
        for path in &removed {
            refuse_changed(&repo, path, removal.cached)?;
        }
    }
    // every file is found before anything changes, so a refused one leaves the index and the disk as they were
    let mut files = Vec::new();
    if !removal.cached {
        for path in &removed {
            files.extend(landing(&place.toplevel, path)?);
        }
    }
    for path in &removed {
        index
            .remove_path(Path::new(path))
            .map_err(|e| failed(&format!("remove {path} from the index"), &e))?;
    }
    index.write().map_err(|e| failed("write the index", &e))?;
    for file in &files {
        remove_file(&place.toplevel, file)?;
    }
    Ok(removed.into_iter().collect())
}

/// git's pathspecs, parsed and matched by gix and combined as git combines them: a file is named when a spec names
/// it and no exclude (`:!x`) does, and when every spec is an exclude, every file the excludes leave. A spec is a path
/// (a file, or a directory and what is under it, `.` the whole tree), a glob, or magic such as `:(glob)**/*.spec`.
pub(crate) struct Pathspecs {
    texts: Vec<String>,
    all: gix::pathspec::Search,
    /// Each spec that is no exclude, on its own, with its place in `texts`.
    each: Vec<(usize, gix::pathspec::Search)>,
}

/// How a file is named: by which spec, and whether as one under the directory the spec names (`whole`).
pub(crate) struct Named<'a> {
    pub(crate) spec: &'a str,
    pub(crate) whole: bool,
}

pub(crate) fn pathspecs(texts: &[String]) -> Result<Pathspecs, GitError> {
    let refused =
        |text: &str, e: &dyn std::fmt::Display| GitError(format!("{text} is not a pathspec: {e}"));
    let search = |text: &str, specs: Vec<gix::pathspec::Pattern>| {
        gix::pathspec::Search::from_specs(specs, None, Path::new("")).map_err(|e| refused(text, &e))
    };
    let mut patterns = Vec::new();
    for text in texts {
        let mut pattern = gix::pathspec::parse(text.as_bytes(), gix::pathspec::Defaults::default())
            .map_err(|e| refused(text, &e))?;
        pattern
            .normalize(Path::new(""), Path::new(""))
            .map_err(|e| refused(text, &e))?;
        // gix keeps the whole tree (`.`) as the text ".", a prefix no path has: an empty path matches every one
        if pattern.is_nil() {
            pattern = gix::pathspec::Pattern::from_literal(&[], pattern.signature);
        }
        patterns.push(pattern);
    }
    let mut each = Vec::new();
    for (at, pattern) in patterns.iter().enumerate() {
        if !pattern.is_excluded() {
            each.push((at, search(&texts[at], vec![pattern.clone()])?));
        }
    }
    Ok(Pathspecs {
        all: search(&texts.join(" "), patterns)?,
        texts: texts.to_vec(),
        each,
    })
}

fn named_by(search: &mut gix::pathspec::Search, path: &str) -> Option<(usize, bool)> {
    use gix::pathspec::search::MatchKind;
    search
        .pattern_matching_relative_path(path.into(), Some(false), &mut |_, _, _, _| false)
        .filter(|found| !found.is_excluded())
        .map(|found| {
            (
                found.sequence_number,
                matches!(found.kind, MatchKind::Prefix | MatchKind::Always),
            )
        })
}

impl Pathspecs {
    /// Whether the file at `path` (relative to the top) is one the pathspecs name.
    pub(crate) fn matches(&mut self, path: &str) -> bool {
        named_by(&mut self.all, path).is_some()
    }

    /// The spec that names the file at `path`, if one does and no exclude takes it back.
    pub(crate) fn named(&mut self, path: &str) -> Option<Named<'_>> {
        let (at, whole) = named_by(&mut self.all, path)?;
        Some(Named {
            // past the last spec: the whole tree that only excludes leave
            spec: self.texts.get(at).map_or(".", String::as_str),
            whole,
        })
    }

    /// The first spec, excludes aside, that names none of `files` on its own (git's "did not match any files").
    pub(crate) fn unmatched(&mut self, files: &[String]) -> Option<&str> {
        for (at, search) in &mut self.each {
            if !files.iter().any(|file| named_by(search, file).is_some()) {
                return Some(&self.texts[*at]);
            }
        }
        None
    }
}

/// Refused as `git rm` refuses without `-f`: a file whose index differs from `HEAD` and from the disk, or (unless
/// `cached`, which keeps the file) either one. A conflicted file is removed whatever its sides hold.
fn refuse_changed(repo: &git2::Repository, path: &str, cached: bool) -> Result<(), GitError> {
    use git2::Status;
    let status = repo
        .status_file(Path::new(path))
        .map_err(|e| failed(&format!("read {path}'s status"), &e))?;
    if status.contains(Status::CONFLICTED) {
        return Ok(());
    }
    let staged = status.intersects(
        Status::INDEX_NEW
            | Status::INDEX_MODIFIED
            | Status::INDEX_RENAMED
            | Status::INDEX_TYPECHANGE,
    );
    let local = status.intersects(Status::WT_MODIFIED | Status::WT_TYPECHANGE);
    let refused = if cached {
        staged && local
    } else {
        staged || local
    };
    if !refused {
        return Ok(());
    }
    let what = match (staged, local) {
        (true, true) => "staged content different from both the file and HEAD",
        (true, false) => "changes staged in the index",
        _ => "local modifications",
    };
    Err(GitError(format!(
        "{path} has {what}: remove it with specforge git rm -f, or keep the file with specforge git rm --cached"
    )))
}

/// Where the file at `path` under `top` is, when there is one to remove: none when it is gone or is a directory (a
/// submodule's, which git leaves). Each directory on the way must be a real one, never a link that would lead the
/// removal out of the worktree.
fn landing(top: &Path, path: &str) -> Result<Option<std::path::PathBuf>, GitError> {
    let rel = Path::new(path);
    if !rel
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err(GitError(format!("{path} is no path inside the worktree")));
    }
    let mut at = top.to_path_buf();
    let mut parts = rel.components().peekable();
    while let Some(part) = parts.next() {
        at.push(part);
        let Ok(meta) = std::fs::symlink_metadata(&at) else {
            return Ok(None);
        };
        if parts.peek().is_none() {
            return Ok((!meta.is_dir()).then_some(at));
        }
        if meta.file_type().is_symlink() {
            return Err(GitError(format!(
                "{path} is beyond a symbolic link: it is not removed"
            )));
        }
        if !meta.is_dir() {
            return Ok(None);
        }
    }
    Ok(None)
}

/// `file` off the disk, and the directories under `top` it leaves empty, as `git rm` removes them.
fn remove_file(top: &Path, file: &Path) -> Result<(), GitError> {
    std::fs::remove_file(file).map_err(|e| failed(&format!("remove {}", file.display()), &e))?;
    let mut dir = file.to_path_buf();
    while dir.pop() && dir != top && std::fs::remove_dir(&dir).is_ok() {}
    Ok(())
}

/// The filter driver `path`'s attributes name (`filter=lfs`), when they name one: git has no driver of its own, so
/// any is a program the repository's config would run.
fn filter_driver(repo: &git2::Repository, path: &Path) -> Result<Option<String>, GitError> {
    let value = repo
        .get_attr_bytes(path, "filter", git2::AttrCheckFlags::FILE_THEN_INDEX)
        .map_err(|e| failed("read the attributes", &e))?;
    Ok(match git2::AttrValue::from_bytes(value) {
        git2::AttrValue::String(driver) => Some(driver.to_string()),
        git2::AttrValue::Bytes(driver) => Some(String::from_utf8_lossy(driver).into_owned()),
        git2::AttrValue::True | git2::AttrValue::False | git2::AttrValue::Unspecified => None,
    })
}

/// `git add`'s refusals of a path it is given (from the repository's top): one that matches no file on disk or in
/// the index, and an ignored one the index does not track, unless `force`.
fn added(
    place: &GitPlace,
    repo: &git2::Repository,
    index: &git2::Index,
    path: &str,
    force: bool,
) -> Result<(), GitError> {
    let under = format!("{path}/");
    let tracked = index.iter().any(|entry| {
        let at = entry.path.as_slice();
        at == path.as_bytes() || at.starts_with(under.as_bytes())
    });
    if tracked {
        return Ok(());
    }
    if std::fs::symlink_metadata(place.toplevel.join(path)).is_err() {
        return Err(GitError(format!("{path} did not match any files")));
    }
    let ignored = repo
        .is_path_ignored(path)
        .map_err(|e| failed("read the ignore rules", &e))?;
    if ignored && !force {
        return Err(GitError(format!(
            "{path} is ignored by the repository's ignore rules: it is staged only when forced, as with specforge git commit --force"
        )));
    }
    Ok(())
}

pub(crate) fn commit(
    gix_port: &Gix,
    place: &GitPlace,
    message: &str,
    recording: &Recording,
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
    if merging && !recording.only.is_empty() {
        return Err(GitError(
            "a merge, cherry-pick or revert is in progress: it is concluded by a commit of the whole index, not of \
             some paths"
                .into(),
        ));
    }
    let author = match &recording.author_of {
        Some(sha) => Some(author_of(
            &repo
                .find_commit(
                    git2::Oid::from_str(sha.as_str())
                        .map_err(|e| failed("read a commit id", &e))?,
                )
                .map_err(|e| failed(&format!("read {}", sha.short()), &e))?,
        )?),
        None => None,
    };
    let committed = record(
        gix_port,
        place,
        message,
        parents,
        recording.allow_empty || merging,
        author.as_ref(),
        &recording.only,
    )?;
    // as git: a commit ends what the files describe, a squash's SQUASH_MSG too
    conclude(place)?;
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

/// The author of `commit`, its time as git writes one.
pub(crate) fn author_of(commit: &git2::Commit<'_>) -> Result<Author, GitError> {
    let who = commit.author();
    let offset = who.when().offset_minutes().saturating_mul(60);
    // `write_to`, not `Display`: an offset past ±99 hours (a hostile commit's) is an error, never a panic.
    let mut time = Vec::new();
    gix::date::Time::new(who.when().seconds(), offset)
        .write_to(&mut time)
        .map_err(|e| failed("write the commit's author time", &e))?;
    Ok(Author {
        name: String::from_utf8_lossy(who.name_bytes()).into_owned(),
        email: String::from_utf8_lossy(who.email_bytes()).into_owned(),
        time: String::from_utf8_lossy(&time).into_owned(),
    })
}

/// `HEAD`'s tree with the index's entries at or under `only` in place of its own there, as `git commit -- <paths>`
/// records it.
fn only_tree(
    repo: &git2::Repository,
    index: &git2::Index,
    only: &[String],
) -> Result<git2::Oid, GitError> {
    let at = |e: git2::Error| failed("build the commit's tree", &e);
    let under = |path: &[u8]| {
        only.iter().any(|o| {
            o == "*" || path == o.as_bytes() || path.starts_with(format!("{o}/").as_bytes())
        })
    };
    let mut tree = git2::Index::new().map_err(at)?;
    if let Ok(head) = repo.head().and_then(|head| head.peel_to_tree()) {
        tree.read_tree(&head).map_err(at)?;
    }
    let dropped: Vec<std::path::PathBuf> = tree
        .iter()
        .filter(|entry| under(&entry.path))
        .map(|entry| std::path::PathBuf::from(String::from_utf8_lossy(&entry.path).into_owned()))
        .collect();
    for path in dropped {
        tree.remove_path(&path).map_err(at)?;
    }
    for entry in index.iter().filter(|entry| under(&entry.path)) {
        tree.add(&entry).map_err(at)?;
    }
    tree.write_tree_to(repo).map_err(at)
}

/// Record the index as a commit on `HEAD`: its parents `HEAD`'s commit unless `parents` names them (a merge), its
/// author the configured identity unless `author` names one, its tree the index's, or `HEAD`'s with the index's
/// entries at `only`.
pub(crate) fn record(
    gix_port: &Gix,
    place: &GitPlace,
    message: &str,
    parents: Option<Vec<Sha>>,
    allow_empty: bool,
    author: Option<&Author>,
    only: &[String],
) -> Result<Committed, GitError> {
    let tree = {
        let repo = libgit2(place)?;
        let mut index = repo.index().map_err(|e| failed("read the index", &e))?;
        if index.has_conflicts() {
            return Err(GitError(
                "the index holds a conflict: resolve it and stage the files first".into(),
            ));
        }
        let tree = if only.is_empty() {
            index
                .write_tree()
                .map_err(|e| failed("write the index's tree", &e))?
        } else {
            only_tree(&repo, &index, only)?
        };
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

/// A checkout that changes no file the worktree changed: a path it would overwrite refuses it.
pub(crate) fn safe() -> git2::build::CheckoutBuilder<'static> {
    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe();
    checkout
}
