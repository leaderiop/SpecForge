//! The diff `Gix` answers: the trees it compares (a commit's, or the index or worktree built as one) and how a
//! changed file reads.

use std::collections::BTreeMap;
use std::path::Path;

use gix::bstr::ByteSlice as _;
use specforge_effects::git::{
    BLOB_CAP, DiffQuery, DiffStatus, GitError, GitFileDiff, GitHunk, PATCH_CAP, Sha, is_binary,
};

use super::{Reads, kept};
use crate::{failed, id};

/// The tree of the commit `sha` names.
pub(crate) fn tree_of(repo: &gix::Repository, sha: &Sha) -> Result<gix::ObjectId, GitError> {
    let read = |e: &dyn std::fmt::Display| failed(&format!("read {}", sha.short()), e);
    repo.find_commit(id(sha)?)
        .map_err(|e| read(&e))?
        .tree_id()
        .map(|t| t.detach())
        .map_err(|e| read(&e))
}

/// The worktree files a diff's tree holds by blob id: those read from disk, their bytes already taken from the diff's
/// [`Reads`], and those over [`BLOB_CAP`] it names but never reads, hashed as they stream and kept out of memory. A
/// bounded diff (a guest's) hashes none it does not read: those stay as the index has them, in `past` by path, with
/// whether the bound (not the cap) left them unread.
#[derive(Default)]
pub(crate) struct Disk {
    pub(crate) read: std::collections::HashSet<gix::ObjectId>,
    pub(crate) unread: std::collections::HashSet<gix::ObjectId>,
    pub(crate) past: BTreeMap<String, bool>,
}

/// The index as a tree, kept in memory; with `worktree`, each tracked file `paths` keeps as it is on disk instead (its
/// bytes taken from `reads`), and the files over the cap hashed without keeping, or, when `reads` is bounded, those
/// over the cap or past it left as the index has them, never hashed ([`Disk::past`]). A changed file `paths`
/// keeps with more than one name is refused (ADR 0146); one it does not keep stays as the index has it, unread.
// shortcut: no clean filters (autocrlf, LFS) are applied to a worktree file; upgrade when a block diffs one
pub(crate) fn index_tree(
    repo: &gix::Repository,
    index: &gix::index::File,
    worktree: bool,
    paths: &[String],
    reads: &Reads,
) -> Result<(gix::ObjectId, Disk), GitError> {
    let mut disk = Disk::default();
    let mut tree = repo
        .edit_tree(repo.empty_tree().id)
        .map_err(|e| failed("build a tree", &e))?;
    for entry in index.entries() {
        if entry.stage() != gix::index::entry::Stage::Unconflicted {
            continue;
        }
        // a mode no tree entry has is no file to diff
        let Some(kind) = entry.mode.to_tree_entry_mode().map(|mode| mode.kind()) else {
            continue;
        };
        tree.upsert(entry.path(index), kind, entry.id)
            .map_err(|e| failed("build a tree", &e))?;
    }
    if worktree {
        use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
        use gix::status::plumbing::index_as_worktree_with_renames::{Entry, Recorder, Sorting};
        let top = repo
            .workdir()
            .ok_or_else(|| GitError("a bare repository has no working tree".into()))?
            .to_path_buf();
        let mut recorder = Recorder::default();
        let options = gix::status::index_worktree::Options {
            // by path, so a bounded diff reads the same files first on every run
            sorting: Some(Sorting::ByPathCaseSensitive),
            ..Default::default()
        };
        repo.index_worktree_status(
            index,
            Vec::<gix::bstr::BString>::new(),
            &mut recorder,
            StatOnly(reads.bounded()),
            NoSubmodule,
            &mut gix::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
            options,
        )
        .map_err(|e| failed("read the status", &e))?;
        for record in recorder.records {
            let Entry::Modification {
                rela_path, status, ..
            } = record
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
                    if !kept(&rela_path.to_string(), paths) {
                        continue;
                    }
                    let rela = gix::path::from_bstr(rela_path.as_bstr());
                    if under_a_link(&top, &rela) {
                        // as git reads it: gone, and never read through the link
                        tree.remove(rela_path.as_bstr())
                            .map_err(|e| failed("build a tree", &e))?;
                        continue;
                    }
                    let file = top.join(rela);
                    let Some((kind, content)) = on_disk(&file, repo.object_hash(), reads) else {
                        continue;
                    };
                    let blob = match content {
                        OnDisk::SecondName(why) => {
                            return Err(GitError(format!(
                                "{rela_path}: {why}, so it is not diffed: replace it with a plain copy (copy it to a \
                                 new name, then move the copy over it), or diff other paths with -- <path>"
                            )));
                        }
                        OnDisk::Bytes(bytes) => {
                            let id = repo
                                .write_blob(bytes)
                                .map_err(|e| failed("hash a file", &e))?
                                .detach();
                            disk.read.insert(id);
                            id
                        }
                        OnDisk::Over(id) => {
                            disk.unread.insert(id);
                            id
                        }
                        OnDisk::Past { bound } => {
                            disk.past.insert(rela_path.to_string(), bound);
                            continue;
                        }
                    };
                    tree.upsert(rela_path.as_bstr(), kind, blob)
                        .map_err(|e| failed("build a tree", &e))?;
                }
                _ => {}
            }
        }
    }
    tree.write()
        .map(|id| (id.detach(), disk))
        .map_err(|e| failed("build a tree", &e))
}

/// How the status compares a tracked file whose stat changed with its entry: unbounded (a person's diff), as gix does,
/// hashing it when its size is unchanged; bounded (a guest's), never: it is reported changed and read once, counted
/// against the diff's bound, when the tree is built, where an unchanged one hashes back to its entry's blob.
#[derive(Clone, Copy)]
struct StatOnly(bool);

impl gix::status::plumbing::index_as_worktree::traits::CompareBlobs for StatOnly {
    type Output = ();

    fn compare_blobs<'a, 'b>(
        &mut self,
        entry: &gix::index::Entry,
        worktree_blob_size: u64,
        data: impl gix::status::plumbing::index_as_worktree::traits::ReadData<'a>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<()>, gix::status::plumbing::index_as_worktree::Error> {
        if self.0 {
            return Ok(Some(()));
        }
        gix::status::plumbing::index_as_worktree::traits::FastEq.compare_blobs(
            entry,
            worktree_blob_size,
            data,
            buf,
        )
    }
}

/// No submodule is opened: a diff lists none of their changes, so none is worth reading.
#[derive(Clone, Copy)]
struct NoSubmodule;

impl gix::status::plumbing::index_as_worktree::traits::SubmoduleStatus for NoSubmodule {
    type Output = ();
    type Error = std::convert::Infallible;

    fn status(
        &mut self,
        _: &gix::index::Entry,
        _: &gix::bstr::BStr,
    ) -> Result<Option<()>, std::convert::Infallible> {
        Ok(None)
    }
}

/// Whether a directory `rela` leads through under `top` is a link, or no directory at all: git reads a file there as
/// gone, so no read follows a link in a path's parent out of the working tree. A status gives a conflicted or an
/// intent-to-add entry whatever the disk holds, so the walk to it is checked here.
// shortcut: checked before the open, so a local process swapping a directory for a link in between still leads the
// read through it (a cloned repository cannot); upgrade to an openat walk when the workspace takes a libc dependency
fn under_a_link(top: &Path, rela: &Path) -> bool {
    let mut at = top.to_path_buf();
    let Some(parent) = rela.parent() else {
        return false;
    };
    parent.components().any(|part| {
        at.push(part);
        !std::fs::symlink_metadata(&at).is_ok_and(|meta| meta.is_dir())
    })
}

/// What a worktree file gives a tree: its bytes, or, over [`BLOB_CAP`], only its blob id, hashed as it streams; or, for a
/// bounded diff, nothing past the cap or the bound (`bound`: the bound).
enum OnDisk {
    Bytes(Vec<u8>),
    Over(gix::ObjectId),
    Past {
        bound: bool,
    },
    /// More than one name: unread, and why.
    SecondName(std::io::Error),
}

/// A file on disk as a tree holds it: its kind and its content (a link's target); `None` when it is gone or is no
/// regular file.
fn on_disk(
    file: &Path,
    hash: gix::hash::Kind,
    reads: &Reads,
) -> Option<(gix::objs::tree::EntryKind, OnDisk)> {
    use gix::objs::tree::EntryKind;
    let meta = std::fs::symlink_metadata(file).ok()?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(file).ok()?;
        return Some((
            EntryKind::Link,
            OnDisk::Bytes(gix::path::into_bstr(target).into_owned().into()),
        ));
    }
    let kind = if executable(&meta) {
        EntryKind::BlobExecutable
    } else {
        EntryKind::Blob
    };
    let mut opened = specforge_common::regular::open_any(file).ok()?;
    if let Err(why) = specforge_common::regular::refuse_if_linked(&opened) {
        return Some((kind, OnDisk::SecondName(why)));
    }
    let length = opened.metadata().ok()?.len();
    if reads.bounded() && (length > BLOB_CAP || !reads.take(length)) {
        return Some((
            kind,
            OnDisk::Past {
                bound: length <= BLOB_CAP,
            },
        ));
    }
    if length > BLOB_CAP {
        let id = gix::objs::compute_stream_hash(
            hash,
            gix::objs::Kind::Blob,
            &mut opened,
            length,
            &mut gix::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .ok()?;
        return Some((kind, OnDisk::Over(id)));
    }
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(opened, BLOB_CAP), &mut bytes).ok()?;
    Some((kind, OnDisk::Bytes(bytes)))
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

/// One tree change as a diff answers it, read as `query` asks and its bytes taken from `reads` (a worktree file's
/// were when `disk` read it); `None` for a directory or a path the query does not keep.
pub(crate) fn file_diff(
    repo: &gix::Repository,
    change: gix::object::tree::diff::ChangeDetached,
    paths: &[String],
    query: &DiffQuery,
    disk: &Disk,
    reads: &Reads,
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
    // the bytes a side takes to read, or `None` when it is never read (over the cap, or past the bound when hashed); a
    // submodule's commit is no blob: it counts no lines
    let size = |side: Side| -> Result<Option<u64>, GitError> {
        match side {
            Some((_, object)) if disk.unread.contains(&object) => Ok(None),
            Some((_, object)) if disk.read.contains(&object) => Ok(Some(0)),
            Some((mode, object)) if mode.is_blob_or_symlink() => {
                let size = repo
                    .find_header(object)
                    .map_err(|e| failed(&format!("give {object}"), &e))?
                    .size();
                Ok((size <= BLOB_CAP).then_some(size))
            }
            _ => Ok(Some(0)),
        }
    };
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
        truncated: false,
        patch: None,
        hunks: None,
        unread: false,
    };
    let (Some(before), Some(after)) = (size(old)?, size(new)?) else {
        file.truncated = true;
        return Ok(Some(file));
    };
    if !reads.take(file.path.len() as u64 + before + after) {
        file.truncated = true;
        file.unread = true;
        return Ok(Some(file));
    }
    let (Some(before), Some(after)) = (bytes(old)?, bytes(new)?) else {
        return Ok(Some(file));
    };
    if is_binary(&before) || is_binary(&after) {
        file.binary = true;
        return Ok(Some(file));
    }
    use gix::diff::blob::{Algorithm, Diff, InternedInput, UnifiedDiff, unified_diff};
    let input = InternedInput::new(before.as_slice(), after.as_slice());
    let diff = Diff::compute(Algorithm::Histogram, &input);
    file.added = diff.count_additions();
    file.removed = diff.count_removals();
    if query.hunks {
        file.hunks = Some(
            diff.hunks()
                .map(|hunk| GitHunk {
                    before_start: hunk.before.start,
                    before_end: hunk.before.end,
                    after_start: hunk.after.start,
                    after_end: hunk.after.end,
                })
                .collect(),
        );
    }
    if query.patch {
        let text = UnifiedDiff::new(
            &diff,
            &input,
            unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
            unified_diff::ContextSize::symmetrical(3),
        )
        .consume()
        .map_err(|e| failed("write a patch", &e))?;
        file.truncated = text.len() > PATCH_CAP;
        file.patch = Some(cut_at_line(text, PATCH_CAP));
    }
    Ok(Some(file))
}

/// `text` cut after its last whole line within `max` bytes.
fn cut_at_line(mut text: String, max: usize) -> String {
    if text.len() > max {
        let end = text[..text.floor_char_boundary(max)]
            .rfind('\n')
            .map_or(0, |at| at + 1);
        text.truncate(end);
    }
    text
}
