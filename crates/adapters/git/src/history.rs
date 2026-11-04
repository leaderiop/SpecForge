//! The log and the diff `Gix` answers: which commits a log keeps and how a commit reads, and the trees a diff
//! compares and how a changed file reads.

use std::collections::BTreeMap;
use std::path::Path;

use gix::bstr::ByteSlice as _;
use specforge_effects::git::{DiffStatus, GitCommit, GitError, GitFileDiff, GitPerson, Sha};

use crate::{failed, id};

/// The paths a log or a diff keeps, slash-separated with no trailing slash; empty when one of them is the top.
pub(crate) fn wanted_paths(paths: &[String]) -> Vec<String> {
    let paths: Vec<String> = paths
        .iter()
        .map(|p| p.replace('\\', "/").trim_matches('/').to_string())
        .collect();
    if paths.iter().any(|p| p.is_empty() || p == ".") {
        return Vec::new();
    }
    paths
}

/// Whether `location` is at or under one of `paths` (every location when there are none).
fn kept(location: &str, paths: &[String]) -> bool {
    paths.is_empty()
        || paths.iter().any(|p| {
            location == p
                || location
                    .strip_prefix(p.as_str())
                    .is_some_and(|r| r.starts_with('/'))
        })
}

/// What `tree` holds at each of `paths`.
fn entries_at(
    tree: &gix::Tree<'_>,
    paths: &[String],
) -> Vec<Option<(gix::objs::tree::EntryMode, gix::ObjectId)>> {
    paths
        .iter()
        .map(|path| {
            let entry = tree.lookup_entry_by_path(path).ok()??;
            Some((entry.mode(), entry.object_id()))
        })
        .collect()
}

/// Whether `commit` changes something at `paths`. A merge counts when it differs from every parent it follows, as
/// git's default history simplification shows it.
// shortcut: git also prunes the parents a merge is the same as from the walk; upgrade if a block needs that order
pub(crate) fn touches(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
    paths: &[String],
    first_parent: bool,
) -> Result<bool, GitError> {
    let tree = commit.tree().map_err(|e| failed("read a tree", &e))?;
    let mine = entries_at(&tree, paths);
    let parents: Vec<gix::ObjectId> = commit.parent_ids().map(|id| id.detach()).collect();
    let parents = if first_parent {
        &parents[..parents.len().min(1)]
    } else {
        &parents[..]
    };
    if parents.is_empty() {
        return Ok(mine.iter().any(Option::is_some));
    }
    for parent in parents {
        let read = |e: &dyn std::fmt::Display| failed(&format!("read {parent}"), e);
        let theirs = repo
            .find_commit(*parent)
            .map_err(|e| read(&e))?
            .tree()
            .map_err(|e| read(&e))?;
        if entries_at(&theirs, paths) == mine {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A signature as a log prints it.
fn person(signature: gix::actor::SignatureRef<'_>) -> GitPerson {
    let time = signature
        .time()
        .map(|t| t.format_or_unix(gix::date::time::format::ISO8601_STRICT))
        .unwrap_or_default();
    GitPerson {
        name: signature.name.to_str_lossy().trim().to_string(),
        email: signature.email.to_str_lossy().trim().to_string(),
        time,
    }
}

/// A commit as a log answers it.
pub(crate) fn commit_of(commit: &gix::Commit<'_>) -> Result<GitCommit, GitError> {
    let at = |e: &dyn std::fmt::Display| failed(&format!("read {}", commit.id), e);
    let message = commit.message().map_err(|e| at(&e))?;
    let mut trailers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(body) = message.body() {
        for trailer in body.trailers() {
            trailers
                .entry(trailer.token.to_str_lossy().into_owned())
                .or_default()
                .push(trailer.value.to_str_lossy().into_owned());
        }
    }
    Ok(GitCommit {
        sha: Sha::new(commit.id.to_string()),
        parents: commit
            .parent_ids()
            .map(|id| Sha::new(id.to_string()))
            .collect(),
        author: person(commit.author().map_err(|e| at(&e))?),
        committer: person(commit.committer().map_err(|e| at(&e))?),
        subject: message.summary().to_str_lossy().into_owned(),
        body: message
            .body
            .map(|b| b.to_str_lossy().trim().to_string())
            .unwrap_or_default(),
        trailers,
    })
}

/// The tree of the commit `sha` names.
pub(crate) fn tree_of(repo: &gix::Repository, sha: &Sha) -> Result<gix::ObjectId, GitError> {
    let read = |e: &dyn std::fmt::Display| failed(&format!("read {}", sha.short()), e);
    repo.find_commit(id(sha)?)
        .map_err(|e| read(&e))?
        .tree_id()
        .map(|t| t.detach())
        .map_err(|e| read(&e))
}

/// The index as a tree, kept in memory; with `worktree`, each tracked file as it is on disk instead.
// shortcut: no clean filters (autocrlf, LFS) are applied to a worktree file; upgrade when a block diffs one
pub(crate) fn index_tree(
    repo: &gix::Repository,
    worktree: bool,
) -> Result<gix::ObjectId, GitError> {
    use gix::objs::tree::EntryKind;
    let index = repo
        .index_or_empty()
        .map_err(|e| failed("read the index", &e))?;
    let mut tree = repo
        .edit_tree(repo.empty_tree().id)
        .map_err(|e| failed("build a tree", &e))?;
    for entry in index.entries() {
        if entry.stage() != gix::index::entry::Stage::Unconflicted {
            continue;
        }
        let kind = match entry.mode {
            gix::index::entry::Mode::SYMLINK => EntryKind::Link,
            gix::index::entry::Mode::FILE_EXECUTABLE => EntryKind::BlobExecutable,
            gix::index::entry::Mode::COMMIT => EntryKind::Commit,
            _ => EntryKind::Blob,
        };
        tree.upsert(entry.path(&index), kind, entry.id)
            .map_err(|e| failed("build a tree", &e))?;
    }
    if worktree {
        use gix::status::index_worktree::Item;
        use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
        let top = repo
            .workdir()
            .ok_or_else(|| GitError("a bare repository has no working tree".into()))?
            .to_path_buf();
        let items = repo
            .status(gix::progress::Discard)
            .map_err(|e| failed("read the status", &e))?
            .untracked_files(gix::status::UntrackedFiles::None)
            .into_index_worktree_iter(Vec::new())
            .map_err(|e| failed("read the status", &e))?;
        for item in items {
            let Item::Modification {
                rela_path, status, ..
            } = item.map_err(|e| failed("read the status", &e))?
            else {
                continue;
            };
            match status {
                EntryStatus::Change(Change::Removed) => {
                    tree.remove(rela_path.as_bstr())
                        .map_err(|e| failed("build a tree", &e))?;
                }
                EntryStatus::Change(Change::Modification { .. } | Change::Type { .. })
                | EntryStatus::Conflict { .. }
                | EntryStatus::IntentToAdd => {
                    let file = top.join(gix::path::from_bstr(rela_path.as_bstr()));
                    let Some((kind, bytes)) = on_disk(&file) else {
                        continue;
                    };
                    let blob = repo
                        .write_blob(bytes)
                        .map_err(|e| failed("hash a file", &e))?;
                    tree.upsert(rela_path.as_bstr(), kind, blob.detach())
                        .map_err(|e| failed("build a tree", &e))?;
                }
                _ => {}
            }
        }
    }
    tree.write()
        .map(|id| id.detach())
        .map_err(|e| failed("build a tree", &e))
}

/// A file on disk as a tree holds it: its kind and its bytes (a link's target); `None` when it is gone.
fn on_disk(file: &Path) -> Option<(gix::objs::tree::EntryKind, Vec<u8>)> {
    use gix::objs::tree::EntryKind;
    let meta = std::fs::symlink_metadata(file).ok()?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(file).ok()?;
        return Some((
            EntryKind::Link,
            gix::path::into_bstr(target).into_owned().into(),
        ));
    }
    let kind = if executable(&meta) {
        EntryKind::BlobExecutable
    } else {
        EntryKind::Blob
    };
    Some((kind, std::fs::read(file).ok()?))
}

#[cfg(unix)]
fn executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> bool {
    false
}

/// Whether `bytes` are binary as git decides: a NUL in the first 8000.
fn binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(8000)].contains(&0)
}

/// One tree change as a diff answers it; `None` for a directory or a path the query does not keep.
pub(crate) fn file_diff(
    repo: &gix::Repository,
    change: gix::object::tree::diff::ChangeDetached,
    paths: &[String],
    patch: bool,
) -> Result<Option<GitFileDiff>, GitError> {
    use gix::object::tree::diff::ChangeDetached as Change;
    type Side = Option<(gix::objs::tree::EntryMode, gix::ObjectId)>;
    let (path, old_path, status, old, new): (_, _, _, Side, Side) = match change {
        Change::Addition {
            location,
            entry_mode,
            id,
            ..
        } => (
            location,
            None,
            DiffStatus::Added,
            None,
            Some((entry_mode, id)),
        ),
        Change::Deletion {
            location,
            entry_mode,
            id,
            ..
        } => (
            location,
            None,
            DiffStatus::Deleted,
            Some((entry_mode, id)),
            None,
        ),
        Change::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
        } => {
            let status = if previous_entry_mode.kind() == entry_mode.kind() {
                DiffStatus::Modified
            } else {
                DiffStatus::TypeChanged
            };
            (
                location,
                None,
                status,
                Some((previous_entry_mode, previous_id)),
                Some((entry_mode, id)),
            )
        }
        Change::Rewrite {
            source_location,
            source_entry_mode,
            source_id,
            entry_mode,
            id,
            location,
            copy,
            ..
        } => {
            let status = if copy {
                DiffStatus::Copied
            } else {
                DiffStatus::Renamed
            };
            (
                location,
                Some(source_location.to_string()),
                status,
                Some((source_entry_mode, source_id)),
                Some((entry_mode, id)),
            )
        }
    };
    let path = path.to_string();
    let is_tree = |side: &Side| side.is_some_and(|(mode, _)| mode.is_tree());
    if is_tree(&old) || is_tree(&new) {
        return Ok(None);
    }
    if !kept(&path, paths) && !old_path.as_deref().is_some_and(|p| kept(p, paths)) {
        return Ok(None);
    }
    // a submodule's commit is no blob: it counts no lines
    let bytes = |side: Side| -> Result<Option<Vec<u8>>, GitError> {
        match side {
            Some((mode, object)) if mode.is_blob_or_symlink() => repo
                .find_blob(object)
                .map(|mut blob| Some(blob.take_data()))
                .map_err(|e| failed(&format!("give {object}"), &e)),
            Some(_) => Ok(None),
            None => Ok(Some(Vec::new())),
        }
    };
    let mut file = GitFileDiff {
        path,
        old_path,
        status,
        added: 0,
        removed: 0,
        binary: false,
        patch: None,
    };
    let (Some(before), Some(after)) = (bytes(old)?, bytes(new)?) else {
        return Ok(Some(file));
    };
    if binary(&before) || binary(&after) {
        file.binary = true;
        return Ok(Some(file));
    }
    use gix::diff::blob::{Algorithm, Diff, InternedInput, UnifiedDiff, unified_diff};
    let input = InternedInput::new(before.as_slice(), after.as_slice());
    let diff = Diff::compute(Algorithm::Histogram, &input);
    file.added = diff.count_additions();
    file.removed = diff.count_removals();
    if patch {
        let text = UnifiedDiff::new(
            &diff,
            &input,
            unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
            unified_diff::ContextSize::symmetrical(3),
        )
        .consume()
        .map_err(|e| failed("write a patch", &e))?;
        file.patch = Some(text);
    }
    Ok(Some(file))
}
