//! The adapter behind the effect plane's [`Git`] port: gix, in process (ADR 0287, ADR 0335, ADR 0339 D5). The git
//! program is never run, so history reads need nothing installed. A repository is opened per call, apart from the
//! variables that would point the host at another repository or another object store (`GIT_DIR`, `GIT_WORK_TREE`,
//! `GIT_OBJECT_DIRECTORY`, `GIT_CONFIG_*`); a repository another user owns is opened with reduced trust, so its config
//! runs nothing. Locating one keeps git's `GIT_CEILING_DIRECTORIES`, and refuses where `GIT_DIR` or `GIT_WORK_TREE`
//! names another repository than the one around the directory (ADR 0352).
//!
//! No lazy fetch (ADR 0300 D5, amended by ADR 0339): gix never asks a partial clone's promisor remote, so a blob the
//! clone lacks reads as missing on every machine. A tree listing reads no blob at all.

use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice as _;
use gix::sec::trust::DefaultForLevel as _;
use specforge_effects::git::{
    Blob, CommitFile, Committed, DiffQuery, DiffSide, Git, GitBlob, GitBranch, GitBranchRef,
    GitError, GitFileDiff, GitLog, GitPlace, GitRemote, GitStash, GitStatus, GitSubmodule,
    GitWorktree, Located, LogQuery, Merged, Merging, ObjectKind, RENAME_PAIRS, Recording, Removal,
    ResetMode, Restoring, Sha, Sized, Staging, TreeEntry,
};

mod branching;
mod history;
mod locate;
mod merging;
mod racy;
mod redirects;
mod refs;
use refs::{branch_of, local_branch};
mod status;
#[cfg(feature = "testing")]
pub mod testing;
mod write;

use history::{Reads, file_diff, index_tree, tree_of, wanted_paths};

/// The `Git` port read through gix.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gix {
    /// How long a status may take before it is given up (`None`: as long as it takes).
    deadline: Option<std::time::Duration>,
}

impl Gix {
    /// The port whose status is given up after `deadline`, for a hook that must not keep its host waiting (ADR
    /// 0320): an error, never a partial status.
    pub fn within(deadline: std::time::Duration) -> Gix {
        Gix {
            deadline: Some(deadline),
        }
    }
}

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
    let mut repo = gix::open_opts(&place.toplevel, options(trust))
        .map_err(|e| failed("open the repository", &e))?;
    without_drivers(&mut repo)?;
    Ok(repo)
}

/// `repo` with no filter or diff driver: each names a program (`filter.<x>.clean`, `smudge`, `process`,
/// `diff.<x>.textconv`, `command`) that gix would run through `sh` with no deadline when it hashes a worktree file, and
/// a committed or cloned config, or a global LFS install, would run it on every status. Only the in-memory config
/// changes. gix has no fsmonitor and no network here, so `core.fsmonitor`, `core.sshCommand` and credential helpers
/// run nothing either; hooks are run only by `commit`, which a person asks for.
// shortcut: a worktree file whose clean filter changes its bytes (LFS, a custom filter) reads as modified once its
// stat changes, where git, running the filter, sees it clean. Upgrade if a person needs LFS worktrees clean.
fn without_drivers(repo: &mut gix::Repository) -> Result<(), GitError> {
    let mut config = repo.config_snapshot_mut();
    for name in ["filter", "diff"] {
        let drivers: Vec<_> = config
            .sections_and_ids_by_name(name)
            .into_iter()
            .flatten()
            .filter(|(section, _)| section.header().subsection_name().is_some())
            .map(|(_, id)| id)
            .collect();
        for id in drivers {
            config.remove_section_by_id(id);
        }
    }
    config
        .commit()
        .map(drop)
        .map_err(|e| failed("read the config", &e))
}

/// Whether a diff from `old` to `new` with `budget` may search renames by similarity: unbounded (a person's diff), or
/// when the blobs a search would compare, every deleted and every added one, fit the budget, which it reads beside the
/// files' own reads. Past it, renames pair by identical content only, which compares ids and reads no blob.
fn similar(
    repo: &gix::Repository,
    old: &gix::Tree<'_>,
    new: &gix::Tree<'_>,
    budget: Option<u64>,
) -> bool {
    use gix::object::tree::diff::ChangeDetached as Change;
    let Some(budget) = budget else {
        return true;
    };
    // the trees alone: no blob is read to list what was deleted and added
    let Ok(changes) = repo.diff_tree_to_tree(old, new, Some(gix::diff::Options::default())) else {
        return false;
    };
    let (mut deleted, mut added) = (Vec::new(), Vec::new());
    for change in changes {
        match change {
            Change::Deletion { id, entry_mode, .. } if entry_mode.is_blob() => deleted.push(id),
            Change::Addition { id, entry_mode, .. } if entry_mode.is_blob() => added.push(id),
            _ => {}
        }
    }
    if deleted.is_empty() || added.is_empty() {
        return true;
    }
    let mut bytes = 0u64;
    deleted.iter().chain(&added).all(|id| {
        // a blob that cannot be sized cannot be held to the budget either
        repo.find_header(*id).is_ok_and(|header| {
            bytes = bytes.saturating_add(header.size());
            bytes <= budget
        })
    })
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
        locate::locate(dir)
    }

    fn leads_to(&self, place: &GitPlace) -> Vec<PathBuf> {
        redirects::leads_to(place)
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

    fn sorted(&self, place: &GitPlace, commits: &[Sha]) -> Result<Vec<Sha>, GitError> {
        let repo = open(place)?;
        let mut dated = Vec::with_capacity(commits.len());
        let mut seen = std::collections::HashSet::new();
        // a commit named twice is listed once, at its first place
        for sha in commits.iter().filter(|sha| seen.insert(sha.as_str())) {
            let read = |e: &dyn std::fmt::Display| failed(&format!("read {}", sha.short()), e);
            let time = repo
                .find_commit(id(sha)?)
                .map_err(|e| read(&e))?
                .committer()
                .map_err(|e| read(&e))?
                .time()
                .map_err(|e| read(&e))?
                .seconds;
            dated.push((time, sha.clone()));
        }
        // stable: a tie keeps the order given
        dated.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
        Ok(dated.into_iter().map(|(_, sha)| sha).collect())
    }

    fn merge_base(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Option<Sha>, GitError> {
        let repo = open(place)?;
        match repo.merge_base(id(a)?, id(b)?) {
            Ok(base) => Ok(Some(Sha::new(base.to_string()))),
            Err(gix::repository::merge_base::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(failed("find a merge base", &error)),
        }
    }

    fn merge_bases(&self, place: &GitPlace, a: &Sha, b: &Sha) -> Result<Vec<Sha>, GitError> {
        let repo = open(place)?;
        // empty when the two share no history
        let bases = repo
            .merge_bases_many(id(a)?, &[id(b)?])
            .map_err(|error| failed("find the merge bases", &error))?;
        Ok(bases
            .iter()
            .map(|base| Sha::new(base.to_string()))
            .collect())
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

    fn config_path(&self, place: &GitPlace, key: &str) -> Result<Option<PathBuf>, GitError> {
        let repo = open(place)?;
        let config = repo.config_snapshot();
        config
            .trusted_path(key)
            .transpose()
            .map(|path| path.map(std::borrow::Cow::into_owned))
            .map_err(|e| failed(&format!("read the path {key}"), &e))
    }

    fn config_flag(&self, place: &GitPlace, key: &str) -> Result<Option<bool>, GitError> {
        let repo = open(place)?;
        repo.config_snapshot()
            .try_boolean(key)
            .transpose()
            .map_err(|error| failed(&format!("read {key}"), &error))
    }

    fn log(&self, place: &GitPlace, query: &LogQuery) -> Result<GitLog, GitError> {
        history::log(&open(place)?, query)
    }

    fn date(&self, text: &str) -> Result<i64, GitError> {
        gix::date::parse(text, Some(std::time::SystemTime::now()))
            .map(|time| time.seconds)
            .map_err(|e| GitError(format!("{text:?} is no date git reads: {e}")))
    }

    fn diff(&self, place: &GitPlace, query: &DiffQuery) -> Result<Vec<GitFileDiff>, GitError> {
        // the index and the worktree are written as trees kept in memory, so every diff is tree to tree and nothing
        // lands in the object store
        let repo = open(place)?.with_object_memory();
        // no base: the empty tree, what a root commit adds everything to
        let base = match &query.base {
            Some(base) => tree_of(&repo, base)?,
            None => repo.empty_tree().id,
        };
        let paths = wanted_paths(&query.paths);
        // the bound twice over: once for the worktree's files on disk, once for the repository's sides
        let (on_disk, reads) = (Reads::up_to(query.budget), Reads::up_to(query.budget));
        let (head, disk) = match &query.head {
            DiffSide::Commit(sha) => (tree_of(&repo, sha)?, Default::default()),
            DiffSide::Index => index_tree(&repo, false, &paths, &on_disk)?,
            DiffSide::Worktree => index_tree(&repo, true, &paths, &on_disk)?,
        };
        let old = repo
            .find_tree(base)
            .map_err(|e| failed("read a tree", &e))?;
        let new = repo
            .find_tree(head)
            .map_err(|e| failed("read a tree", &e))?;
        // the host's own rename rules, never the repository's `diff.renames` and `diff.renameLimit`: a configured limit
        // of 0 would compare every deletion with every addition; the default options track none
        let rewrites = query.renames.then(|| gix::diff::Rewrites {
            copies: None,
            percentage: similar(&repo, &old, &new, query.budget).then_some(0.5),
            limit: RENAME_PAIRS,
            track_empty: false,
        });
        let options = gix::diff::Options::default().with_rewrites(rewrites);
        let changes = repo
            .diff_tree_to_tree(&old, &new, Some(options))
            .map_err(|e| failed("diff two trees", &e))?;
        let mut files = Vec::new();
        for change in changes {
            if let Some(file) = file_diff(&repo, change, &paths, query, &disk, &reads)? {
                files.push(file);
            }
        }
        // a worktree file a bounded diff did not read: changed on disk, so listed, with no counts
        for (path, bound) in disk.past {
            let file = match files.iter_mut().find(|file| file.path == path) {
                Some(file) => file,
                None => {
                    files.push(GitFileDiff {
                        path,
                        old_path: None,
                        status: specforge_effects::git::DiffStatus::Modified,
                        added: 0,
                        removed: 0,
                        binary: false,
                        truncated: false,
                        patch: None,
                        hunks: None,
                        unread: false,
                    });
                    files.last_mut().expect("just pushed")
                }
            };
            (file.added, file.removed, file.patch, file.hunks) = (0, 0, None, None);
            (file.truncated, file.unread) = (true, bound);
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn blob(&self, place: &GitPlace, revision: &str) -> Result<Option<GitBlob>, GitError> {
        let repo = open(place)?;
        let Ok(id) = repo.rev_parse_single(revision) else {
            return Ok(None);
        };
        // the header first: a blob over the cap is sized, never inflated
        let header = repo
            .find_header(id)
            .map_err(|e| failed(&format!("read {id}"), &e))?;
        if header.kind() != gix::objs::Kind::Blob {
            return Ok(None);
        }
        let bytes = if header.size() > specforge_effects::git::BLOB_CAP {
            None
        } else {
            let object = id.object().map_err(|e| failed(&format!("read {id}"), &e))?;
            Some(object.detach().data)
        };
        Ok(Some(GitBlob {
            sha: Sha::new(id.to_string()),
            size: header.size(),
            bytes,
        }))
    }

    fn branches(&self, place: &GitPlace, remote: bool) -> Result<Vec<GitBranchRef>, GitError> {
        refs::branches(&open(place)?, remote)
    }

    fn worktrees(&self, place: &GitPlace) -> Result<Vec<GitWorktree>, GitError> {
        refs::worktrees(&open(place)?)
    }

    fn superproject(&self, place: &GitPlace) -> Result<Option<PathBuf>, GitError> {
        let Some(parent) = place.toplevel.parent() else {
            return Ok(None);
        };
        let Located::Repo(outer) = self.locate(parent) else {
            return Ok(None);
        };
        let Some(rel) = place
            .toplevel
            .strip_prefix(&outer.toplevel)
            .ok()
            .and_then(tree_path)
        else {
            return Ok(None);
        };
        let index = open(&outer)?
            .index_or_empty()
            .map_err(|e| failed("read the index", &e))?;
        let submodule = index
            .entry_by_path(rel.as_str().into())
            .is_some_and(|entry| entry.mode == gix::index::entry::Mode::COMMIT);
        Ok(submodule.then_some(outer.toplevel))
    }

    fn submodules(&self, place: &GitPlace) -> Result<Vec<GitSubmodule>, GitError> {
        refs::submodules(&open(place)?)
    }

    fn remotes(&self, place: &GitPlace) -> Result<Vec<GitRemote>, GitError> {
        Ok(refs::remotes(&open(place)?))
    }

    fn writable(&self, place: &GitPlace) -> Result<(), GitError> {
        write::writable(place)
    }

    fn write_commit_message(&self, place: &GitPlace, message: &str) -> Result<PathBuf, GitError> {
        write::write_commit_message(place, message)
    }

    fn read_commit_message(&self, place: &GitPlace) -> Result<String, GitError> {
        write::read_commit_message(place)
    }

    fn stage(&self, place: &GitPlace, staging: &Staging) -> Result<(), GitError> {
        write::stage(place, staging)
    }

    fn remove(
        &self,
        place: &GitPlace,
        paths: &[String],
        removal: Removal,
    ) -> Result<Vec<String>, GitError> {
        write::remove(place, paths, removal)
    }

    fn commit_index(
        &self,
        place: &GitPlace,
        message: &str,
        recording: &Recording,
    ) -> Result<Committed, GitError> {
        write::commit(self, place, message, recording)
    }

    fn message(&self, place: &GitPlace, commit: &Sha) -> Result<String, GitError> {
        let repo = open(place)?;
        let found = repo
            .find_commit(id(commit)?)
            .map_err(|e| failed(&format!("read {}", commit.short()), &e))?;
        let message = found
            .message_raw()
            .map_err(|e| failed(&format!("read {}", commit.short()), &e))?;
        Ok(String::from_utf8_lossy(message).into_owned())
    }

    fn branch_create(&self, place: &GitPlace, name: &str, at: &Sha) -> Result<(), GitError> {
        branching::branch_create(place, name, at)
    }

    fn branch_delete(&self, place: &GitPlace, name: &str) -> Result<Sha, GitError> {
        branching::branch_delete(place, name)
    }

    fn tag(
        &self,
        place: &GitPlace,
        name: &str,
        at: &Sha,
        message: Option<&str>,
    ) -> Result<Sha, GitError> {
        branching::tag(place, name, at, message)
    }

    fn switch(&self, place: &GitPlace, name: &str) -> Result<(), GitError> {
        racy::kept(place, || branching::switch(place, name))
    }

    fn worktree_add(&self, place: &GitPlace, path: &Path, branch: &str) -> Result<(), GitError> {
        branching::worktree_add(place, path, branch)
    }

    fn worktree_remove(&self, place: &GitPlace, path: &Path) -> Result<(), GitError> {
        branching::worktree_remove(place, path)
    }

    fn init(&self, dir: &Path) -> Result<bool, GitError> {
        branching::init(dir)
    }

    fn clone_local(&self, source: &GitPlace, into: &Path) -> Result<(), GitError> {
        branching::clone_local(source, into)
    }

    fn worktree_lock(
        &self,
        place: &GitPlace,
        path: &Path,
        lock: bool,
        reason: Option<&str>,
    ) -> Result<(), GitError> {
        branching::worktree_lock(place, path, lock, reason)
    }

    fn worktree_prune(&self, place: &GitPlace) -> Result<Vec<PathBuf>, GitError> {
        branching::worktree_prune(place)
    }

    fn merge(&self, place: &GitPlace, theirs: &Sha, merging: &Merging) -> Result<Merged, GitError> {
        racy::kept(place, || merging::merge(self, place, theirs, merging))
    }

    fn reset(&self, place: &GitPlace, to: &Sha, mode: ResetMode) -> Result<(), GitError> {
        racy::kept(place, || merging::reset(place, to, mode))
    }

    fn restore(
        &self,
        place: &GitPlace,
        paths: &[String],
        restoring: &Restoring,
    ) -> Result<(), GitError> {
        racy::kept(place, || merging::restore(place, paths, restoring))
    }

    fn stash_push(
        &self,
        place: &GitPlace,
        message: Option<&str>,
        untracked: bool,
    ) -> Result<Sha, GitError> {
        racy::kept(place, || merging::stash_push(place, message, untracked))
    }

    fn stash_pop(&self, place: &GitPlace, index: usize) -> Result<(), GitError> {
        racy::kept(place, || merging::stash_pop(place, index))
    }

    fn stashes(&self, place: &GitPlace) -> Result<Vec<GitStash>, GitError> {
        merging::stashes(place)
    }

    fn pick(
        &self,
        place: &GitPlace,
        commit: &Sha,
        revert: bool,
        record: bool,
    ) -> Result<Merged, GitError> {
        racy::kept(place, || {
            if record {
                merging::pick(self, place, commit, revert)
            } else {
                merging::pick_staged(place, commit, revert)
            }
        })
    }

    fn pick_end(&self, place: &GitPlace, abort: bool) -> Result<bool, GitError> {
        racy::kept(place, || merging::pick_end(place, abort))
    }

    fn names_a_remote(&self, source: &str) -> bool {
        // A plain path parses as a `file` URL too: it serializes in its alternative form, `file://` does not.
        gix::url::parse(source.into()).is_ok_and(|url| {
            url.scheme != gix::url::Scheme::File || !url.serialize_alternative_form
        })
    }

    fn status(&self, place: &GitPlace) -> Result<GitStatus, GitError> {
        status::read(place, self.deadline, gix::status::UntrackedFiles::Collapsed)
    }

    fn status_by_file(&self, place: &GitPlace) -> Result<GitStatus, GitError> {
        status::read(place, self.deadline, gix::status::UntrackedFiles::Files)
    }
}
