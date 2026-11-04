//! The adapter behind the effect plane's [`Git`] port: gix, in process (ADR 0287, ADR 0335, ADR 0339 D5). The git
//! program is never run, so history reads need nothing installed. A repository is opened per call, apart from the
//! variables that would point the host at another repository or another object store (`GIT_DIR`, `GIT_WORK_TREE`,
//! `GIT_OBJECT_DIRECTORY`, `GIT_CONFIG_*`); a repository another user owns is opened with reduced trust, so its config
//! runs nothing.
//!
//! No lazy fetch (ADR 0300 D5, amended by ADR 0339): gix never asks a partial clone's promisor remote, so a blob the
//! clone lacks reads as missing on every machine. A tree listing reads no blob at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice as _;
use gix::sec::trust::DefaultForLevel as _;
use specforge_effects::git::{
    Blob, CommitFile, DiffQuery, DiffSide, Git, GitBranch, GitBranchRef, GitChange, GitCommit,
    GitEntry, GitEntryKind, GitError, GitFileDiff, GitPlace, GitRemote, GitStatus, GitWorktree,
    Located, LogQuery, ObjectKind, Sha, Sized, TreeEntry,
};

mod history;
mod refs;
#[cfg(feature = "testing")]
pub mod testing;

use history::{commit_of, file_diff, index_tree, touches, tree_of, wanted_paths};

/// The `Git` port read through gix.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gix;

/// How a repository of `trust` is opened: as gix opens one of that trust, minus every environment variable that names
/// another repository, object store or configuration.
fn options(trust: gix::sec::Trust) -> gix::open::Options {
    let mut permissions = gix::open::Permissions::default_for_level(trust);
    permissions.env.git_prefix = gix::sec::Permission::Deny;
    permissions.env.objects = gix::sec::Permission::Deny;
    permissions.config.env = false;
    gix::open::Options::default_for_level(trust).permissions(permissions)
}

/// The trust map discovery opens a found repository with.
fn trust_map() -> gix::sec::trust::Mapping<gix::open::Options> {
    gix::sec::trust::Mapping {
        full: options(gix::sec::Trust::Full),
        reduced: options(gix::sec::Trust::Reduced),
    }
}

/// The repository at `place`.
fn open(place: &GitPlace) -> Result<gix::Repository, GitError> {
    let trust =
        gix::sec::Trust::from_path_ownership(&place.git_dir).unwrap_or(gix::sec::Trust::Reduced);
    gix::open_opts(&place.toplevel, options(trust)).map_err(|e| failed("open the repository", &e))
}

/// A gix failure as the port's error.
fn failed(what: &str, error: &dyn std::fmt::Display) -> GitError {
    GitError(format!("git cannot {what}: {error}"))
}

/// The object `sha` names.
fn id(sha: &Sha) -> Result<gix::ObjectId, GitError> {
    gix::ObjectId::from_hex(sha.as_str().as_bytes()).map_err(|e| failed("read a commit id", &e))
}

/// `path` as a slash-separated tree path; `None` when it is not UTF-8 (a tree path the host could not name back).
fn tree_path(path: &Path) -> Option<String> {
    path.to_str().map(|p| p.replace('\\', "/"))
}

/// The tree of `commit`, and its subtree at `dir` (`None` when the commit holds no directory there).
fn tree_under<'r>(
    repo: &'r gix::Repository,
    commit: &Sha,
    dir: &Path,
) -> Result<Option<gix::Tree<'r>>, GitError> {
    let tree = repo
        .find_commit(id(commit)?)
        .map_err(|e| failed(&format!("read {}", commit.short()), &e))?
        .tree()
        .map_err(|e| failed(&format!("read the tree of {}", commit.short()), &e))?;
    if dir.as_os_str().is_empty() {
        return Ok(Some(tree));
    }
    let dir = tree_path(dir).ok_or_else(|| {
        GitError("the project's directory is not valid UTF-8, so git cannot list it".into())
    })?;
    let Some(entry) = tree
        .lookup_entry_by_path(&dir)
        .map_err(|e| failed("read a tree", &e))?
    else {
        return Ok(None);
    };
    if !entry.mode().is_tree() {
        return Ok(None);
    }
    entry
        .object()
        .map_err(|e| failed("read a tree", &e))
        .map(|o| Some(o.into_tree()))
}

/// One entry of a commit's tree: its path from the top, its mode and its object.
type Listed = (PathBuf, gix::objs::tree::EntryMode, gix::ObjectId);

/// Every entry under `dir` of `commit` but its directories.
fn listing(repo: &gix::Repository, commit: &Sha, dir: &Path) -> Result<Vec<Listed>, GitError> {
    let Some(tree) = tree_under(repo, commit, dir)? else {
        return Ok(Vec::new());
    };
    let mut recorder = gix::traverse::tree::Recorder::default();
    tree.traverse()
        .breadthfirst(&mut recorder)
        .map_err(|e| failed("list a tree", &e))?;
    Ok(recorder
        .records
        .into_iter()
        .filter(|entry| !entry.mode.is_tree())
        .map(|entry| {
            let rel = PathBuf::from(entry.filepath.to_str_lossy().into_owned());
            (dir.join(rel), entry.mode, entry.oid)
        })
        .collect())
}

/// The object at `path` (from the top) in a commit's `tree`, when it holds a blob there (a file or a link).
fn blob_at(tree: &gix::Tree<'_>, path: &Path) -> Option<gix::ObjectId> {
    let entry = tree.lookup_entry_by_path(tree_path(path)?).ok()??;
    entry.mode().is_blob_or_symlink().then(|| entry.object_id())
}

impl Git for Gix {
    fn locate(&self, dir: &Path) -> Located {
        if !dir.is_dir() {
            return Located::NotARepository;
        }
        use gix::discover::upwards::Error as Upwards;
        let repo =
            match gix::ThreadSafeRepository::discover_opts(dir, Default::default(), trust_map()) {
                Ok(repo) => repo.to_thread_local(),
                Err(gix::discover::Error::Discover(
                    Upwards::NoGitRepository { .. }
                    | Upwards::NoGitRepositoryWithinCeiling { .. }
                    | Upwards::NoGitRepositoryWithinFs { .. },
                )) => return Located::NotARepository,
                Err(error) => return Located::Unavailable(error.to_string()),
            };
        let Some(top) = repo.workdir() else {
            return Located::Unavailable("a bare repository has no working tree".into());
        };
        let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| dir.join(p));
        Located::Repo(GitPlace {
            toplevel: real(top),
            git_dir: real(repo.git_dir()),
            common_dir: real(repo.common_dir()),
            shallow: repo.is_shallow(),
        })
    }

    fn commit(&self, place: &GitPlace, revision: &str) -> Result<Option<Sha>, GitError> {
        let repo = open(place)?;
        // a revision that names nothing, or no commit (an unborn HEAD, a blob), is no commit
        Ok(repo
            .rev_parse_single(revision)
            .ok()
            .and_then(|id| id.object().ok()?.peel_to_commit().ok())
            .map(|commit| Sha::new(commit.id.to_string())))
    }

    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, GitError> {
        let repo = open(place)?;
        match repo.merge_base(id(a)?, id(b)?) {
            Ok(base) => Ok(Some(Sha::new(base.to_string()))),
            Err(gix::repository::merge_base::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(failed("find a merge base", &error)),
        }
    }

    fn files(
        &self,
        place: &GitPlace,
        commit: &Sha,
        dir: &Path,
        wanted: &dyn Fn(&Path) -> bool,
    ) -> Result<Vec<CommitFile>, GitError> {
        let repo = open(place)?;
        let mut files = Vec::new();
        // only what the caller wants, so a repository's images, lockfiles and vendored trees are never read
        for (path, mode, object) in listing(&repo, commit, dir)? {
            if mode.is_blob() && wanted(&path) {
                let mut blob = repo
                    .find_blob(object)
                    .map_err(|e| failed(&format!("give {object}"), &e))?;
                files.push((path, blob.take_data()));
            }
        }
        Ok(files)
    }

    fn tree(&self, place: &GitPlace, commit: &Sha, dir: &Path) -> Result<Vec<TreeEntry>, GitError> {
        let repo = open(place)?;
        Ok(listing(&repo, commit, dir)?
            .into_iter()
            .map(|(path, mode, _)| (path, mode.is_blob()))
            .collect())
    }

    fn sizes(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Sized>, GitError> {
        let repo = open(place)?;
        let Some(tree) = tree_under(&repo, commit, Path::new(""))? else {
            return Ok(paths.iter().map(|p| (p.clone(), None)).collect());
        };
        // a blob the clone lacks has no header: missing, never fetched
        Ok(paths
            .iter()
            .map(|path| {
                let size = blob_at(&tree, path)
                    .and_then(|object| repo.find_header(object).ok())
                    .map(|header| header.size());
                (path.clone(), size)
            })
            .collect())
    }

    fn blobs(
        &self,
        place: &GitPlace,
        commit: &Sha,
        paths: &[PathBuf],
    ) -> Result<Vec<Blob>, GitError> {
        let repo = open(place)?;
        let Some(tree) = tree_under(&repo, commit, Path::new(""))? else {
            return Ok(paths.iter().map(|p| (p.clone(), None)).collect());
        };
        Ok(paths
            .iter()
            .map(|path| {
                let bytes = blob_at(&tree, path)
                    .and_then(|object| repo.find_blob(object).ok())
                    .map(|mut blob| blob.take_data());
                (path.clone(), bytes)
            })
            .collect())
    }

    fn branch(&self, place: &GitPlace) -> Result<String, GitError> {
        let repo = open(place)?;
        Ok(local_branch(&repo)?.unwrap_or_default())
    }

    fn head(&self, place: &GitPlace) -> Result<GitBranch, GitError> {
        branch_of(&open(place)?)
    }

    fn resolve(
        &self,
        place: &GitPlace,
        revision: &str,
    ) -> Result<Option<(Sha, ObjectKind)>, GitError> {
        let repo = open(place)?;
        let Ok(id) = repo.rev_parse_single(revision) else {
            return Ok(None);
        };
        let kind = match id
            .header()
            .map_err(|e| failed(&format!("read {id}"), &e))?
            .kind()
        {
            gix::objs::Kind::Commit => ObjectKind::Commit,
            gix::objs::Kind::Tree => ObjectKind::Tree,
            gix::objs::Kind::Blob => ObjectKind::Blob,
            gix::objs::Kind::Tag => ObjectKind::Tag,
        };
        Ok(Some((Sha::new(id.to_string()), kind)))
    }

    fn config(&self, place: &GitPlace, key: &str) -> Result<Vec<String>, GitError> {
        let repo = open(place)?;
        let config = repo.config_snapshot();
        Ok(config
            .plumbing()
            .strings(key)
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.to_str_lossy().into_owned())
            .collect())
    }

    fn log(&self, place: &GitPlace, query: &LogQuery) -> Result<Vec<GitCommit>, GitError> {
        use gix::revision::walk::Sorting;
        use gix::traverse::commit::simple::CommitTimeOrder;
        let repo = open(place)?;
        let tips = query.tips.iter().map(id).collect::<Result<Vec<_>, _>>()?;
        let hidden = query.hidden.iter().map(id).collect::<Result<Vec<_>, _>>()?;
        let mut walk = repo
            .rev_walk(tips)
            .with_hidden(hidden)
            .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst));
        if query.first_parent {
            walk = walk.first_parent_only();
        }
        let walk = walk.all().map_err(|e| failed("walk the history", &e))?;
        let paths = wanted_paths(&query.paths);
        let mut commits = Vec::new();
        for info in walk {
            if query.max.is_some_and(|max| commits.len() >= max) {
                break;
            }
            let info = info.map_err(|e| failed("walk the history", &e))?;
            let commit = repo
                .find_commit(info.id)
                .map_err(|e| failed(&format!("read {}", info.id), &e))?;
            if !paths.is_empty() && !touches(&repo, &commit, &paths, query.first_parent)? {
                continue;
            }
            commits.push(commit_of(&commit)?);
        }
        Ok(commits)
    }

    fn diff(&self, place: &GitPlace, query: &DiffQuery) -> Result<Vec<GitFileDiff>, GitError> {
        // the index and the worktree are written as trees kept in memory, so every diff is tree to tree and nothing
        // lands in the object store
        let repo = open(place)?.with_object_memory();
        let base = tree_of(&repo, &query.base)?;
        let head = match &query.head {
            DiffSide::Commit(sha) => tree_of(&repo, sha)?,
            DiffSide::Index => index_tree(&repo, false)?,
            DiffSide::Worktree => index_tree(&repo, true)?,
        };
        let old = repo
            .find_tree(base)
            .map_err(|e| failed("read a tree", &e))?;
        let new = repo
            .find_tree(head)
            .map_err(|e| failed("read a tree", &e))?;
        let changes = repo
            .diff_tree_to_tree(&old, &new, None)
            .map_err(|e| failed("diff two trees", &e))?;
        let paths = wanted_paths(&query.paths);
        let mut files = Vec::new();
        for change in changes {
            if let Some(file) = file_diff(&repo, change, &paths, query.patch)? {
                files.push(file);
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn blob(&self, place: &GitPlace, revision: &str) -> Result<Option<(Sha, Vec<u8>)>, GitError> {
        let repo = open(place)?;
        let Ok(id) = repo.rev_parse_single(revision) else {
            return Ok(None);
        };
        let object = id.object().map_err(|e| failed(&format!("read {id}"), &e))?;
        if object.kind != gix::objs::Kind::Blob {
            return Ok(None);
        }
        Ok(Some((Sha::new(id.to_string()), object.detach().data)))
    }

    fn branches(&self, place: &GitPlace, remote: bool) -> Result<Vec<GitBranchRef>, GitError> {
        refs::branches(&open(place)?, remote)
    }

    fn worktrees(&self, place: &GitPlace) -> Result<Vec<GitWorktree>, GitError> {
        refs::worktrees(&open(place)?)
    }

    fn remotes(&self, place: &GitPlace) -> Result<Vec<GitRemote>, GitError> {
        Ok(refs::remotes(&open(place)?))
    }

    fn status(&self, place: &GitPlace) -> Result<GitStatus, GitError> {
        let repo = open(place)?;
        let branch = branch_of(&repo)?;
        let items = repo
            .status(gix::progress::Discard)
            .map_err(|e| failed("read the status", &e))?
            .untracked_files(gix::status::UntrackedFiles::Collapsed)
            .index_worktree_rewrites(None)
            .into_iter(None)
            .map_err(|e| failed("read the status", &e))?;
        let mut tracked: BTreeMap<String, GitEntry> = BTreeMap::new();
        let mut untracked = Vec::new();
        for item in items {
            let item = item.map_err(|e| failed("read the status", &e))?;
            note(&mut tracked, &mut untracked, item);
        }
        Ok(status_of(branch, tracked, untracked))
    }
}

/// The status `tracked` and `untracked` make, in git's order: the tracked entries by path, then the untracked paths.
fn status_of(
    branch: GitBranch,
    tracked: BTreeMap<String, GitEntry>,
    mut untracked: Vec<String>,
) -> GitStatus {
    let mut entries: Vec<GitEntry> = tracked
        .into_values()
        .map(|mut entry| {
            // a conflict is one line whatever else the index says of its path
            if entry.kind == GitEntryKind::Unmerged {
                entry.index = Some(GitChange::Unmerged);
                entry.worktree = Some(GitChange::Unmerged);
                entry.orig_path = None;
            }
            entry
        })
        .collect();
    untracked.sort();
    entries.extend(untracked.into_iter().map(|path| GitEntry {
        kind: GitEntryKind::Untracked,
        path,
        orig_path: None,
        index: None,
        worktree: None,
    }));
    GitStatus { branch, entries }
}

/// The branch `HEAD` is on, born or not; `None` when it is detached.
fn local_branch(repo: &gix::Repository) -> Result<Option<String>, GitError> {
    let name = repo.head_name().map_err(|e| failed("read HEAD", &e))?;
    Ok(name
        .filter(|name| name.category() == Some(gix::refs::Category::LocalBranch))
        .map(|name| name.shorten().to_string()))
}

/// `status`'s branch headers: `HEAD`'s commit and branch, its upstream, and how far each is ahead of the other.
fn branch_of(repo: &gix::Repository) -> Result<GitBranch, GitError> {
    let oid = repo.head_id().ok().map(|id| id.detach());
    let mut branch = GitBranch {
        oid: oid.map(|id| id.to_string()),
        head: local_branch(repo)?,
        ..GitBranch::default()
    };
    if let Some(name) = repo.head_name().map_err(|e| failed("read HEAD", &e))? {
        (branch.upstream, branch.ahead, branch.behind) = upstream_of(repo, name.as_ref(), oid)?;
    }
    Ok(branch)
}

/// The upstream of the local branch `name` at `tip`, and how many commits each is ahead of the other.
fn upstream_of(
    repo: &gix::Repository,
    name: &gix::refs::FullNameRef,
    tip: Option<gix::ObjectId>,
) -> Result<(Option<String>, u32, u32), GitError> {
    let Some(Ok(tracking)) =
        repo.branch_remote_tracking_ref_name(name, gix::remote::Direction::Fetch)
    else {
        return Ok((None, 0, 0));
    };
    let upstream = repo
        .try_find_reference(tracking.as_ref())
        .ok()
        .flatten()
        .and_then(|mut r| r.peel_to_id().ok())
        .map(|id| id.detach());
    let (ahead, behind) = match (tip, upstream) {
        (Some(tip), Some(upstream)) => (
            count_hiding(repo, tip, upstream)?,
            count_hiding(repo, upstream, tip)?,
        ),
        _ => (0, 0),
    };
    Ok((Some(tracking.shorten().to_string()), ahead, behind))
}

/// How many commits `tip` reaches that `hidden` does not.
fn count_hiding(
    repo: &gix::Repository,
    tip: gix::ObjectId,
    hidden: gix::ObjectId,
) -> Result<u32, GitError> {
    let walk = repo
        .rev_walk([tip])
        .with_hidden([hidden])
        .all()
        .map_err(|e| failed("walk the history", &e))?;
    let mut count = 0u32;
    for info in walk {
        info.map_err(|e| failed("walk the history", &e))?;
        count = count.saturating_add(1);
    }
    Ok(count)
}

/// The tracked entry of `path`, a plain change until an item says more.
fn entry_at<'a>(
    tracked: &'a mut BTreeMap<String, GitEntry>,
    path: &gix::bstr::BStr,
) -> &'a mut GitEntry {
    tracked.entry(path.to_string()).or_insert_with(|| GitEntry {
        kind: GitEntryKind::Changed,
        path: path.to_string(),
        orig_path: None,
        index: None,
        worktree: None,
    })
}

/// One status item onto the tracked entries (by path) or the untracked paths.
fn note(
    tracked: &mut BTreeMap<String, GitEntry>,
    untracked: &mut Vec<String>,
    item: gix::status::Item,
) {
    use gix::diff::index::Change as Staged;
    use gix::status::index_worktree::Item as Worktree;
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
    match item {
        gix::status::Item::TreeIndex(change) => match change {
            Staged::Addition { location, .. } => {
                entry_at(tracked, location.as_ref()).index = Some(GitChange::Added)
            }
            Staged::Deletion { location, .. } => {
                entry_at(tracked, location.as_ref()).index = Some(GitChange::Deleted)
            }
            Staged::Modification {
                location,
                previous_entry_mode,
                entry_mode,
                ..
            } => {
                let kind =
                    |mode: gix::index::entry::Mode| mode.to_tree_entry_mode().map(|m| m.kind());
                entry_at(tracked, location.as_ref()).index =
                    Some(if kind(previous_entry_mode) != kind(entry_mode) {
                        GitChange::TypeChanged
                    } else {
                        GitChange::Modified
                    });
            }
            Staged::Rewrite {
                source_location,
                location,
                ..
            } => {
                let entry = entry_at(tracked, location.as_ref());
                entry.kind = GitEntryKind::Renamed;
                entry.orig_path = Some(source_location.to_string());
                entry.index = Some(GitChange::Renamed);
            }
        },
        gix::status::Item::IndexWorktree(Worktree::Modification {
            rela_path, status, ..
        }) => {
            let change = match status {
                EntryStatus::Conflict { .. } => {
                    entry_at(tracked, rela_path.as_ref()).kind = GitEntryKind::Unmerged;
                    return;
                }
                EntryStatus::Change(Change::Removed) => GitChange::Deleted,
                EntryStatus::Change(Change::Type { .. }) => GitChange::TypeChanged,
                EntryStatus::Change(
                    Change::Modification { .. } | Change::SubmoduleModification(_),
                ) => GitChange::Modified,
                EntryStatus::IntentToAdd => GitChange::Added,
                EntryStatus::NeedsUpdate(_) => return,
            };
            entry_at(tracked, rela_path.as_ref()).worktree = Some(change);
        }
        gix::status::Item::IndexWorktree(Worktree::DirectoryContents { entry, .. }) => {
            if entry.status == gix::dir::entry::Status::Untracked {
                let mut path = entry.rela_path.to_string();
                if entry.disk_kind == Some(gix::dir::entry::Kind::Directory) {
                    path.push('/');
                }
                untracked.push(path);
            }
        }
        gix::status::Item::IndexWorktree(Worktree::Rewrite { .. }) => {}
    }
}
