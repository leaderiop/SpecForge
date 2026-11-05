//! The worktree's status as `git status --porcelain=v2` reads it: gix's status items folded into one entry per path.

use std::collections::BTreeMap;

use specforge_effects::git::{
    GitBranch, GitChange, GitEntry, GitEntryKind, GitError, GitPlace, GitStatus,
};

use crate::refs::branch_of;
use crate::{failed, open};

/// The status of the worktree at `place`, given up after `deadline` when one is set.
pub(crate) fn read(
    place: &GitPlace,
    deadline: Option<std::time::Duration>,
) -> Result<GitStatus, GitError> {
    let repo = open(place)?;
    let branch = branch_of(&repo)?;
    let started = std::time::Instant::now();
    let interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // The timer ends when the status does (its sender dropped), or raises `interrupt` at the deadline.
    let (done, timer) = std::sync::mpsc::channel::<()>();
    if let Some(deadline) = deadline {
        let raise = std::sync::Arc::clone(&interrupt);
        std::thread::spawn(move || {
            if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = timer.recv_timeout(deadline) {
                raise.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });
    }
    let items = repo
        .status(gix::progress::Discard)
        .map_err(|e| failed("read the status", &e))?
        .should_interrupt_owned(std::sync::Arc::clone(&interrupt))
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
    drop(done);
    let late = deadline.is_some_and(|d| started.elapsed() >= d);
    if late || interrupt.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(GitError(format!(
            "git gave up the status after {:?}",
            deadline.unwrap_or_default()
        )));
    }
    Ok(status_of(branch, tracked, untracked))
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
