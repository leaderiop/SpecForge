//! git's racy-clean rule kept across every index write, staging's and the ones libgit2 makes on its own (a checkout,
//! a reset, a merge): libgit2 judges an entry racy in nanoseconds, but git and gix compare a file's stat with its
//! entry's to the second, so an entry libgit2 leaves unsmudged can read clean after a same-size rewrite in the second
//! it was stamped.

use std::path::Path;

use specforge_effects::git::{GitError, GitPlace};

use crate::failed;

/// `index` written with git's racy-clean rule kept: an entry staged in or after the second its file was last written
/// whose file now differs is smudged (size 0) first, so every reader compares that file's content. libgit2 judges this
/// in nanoseconds, but git and gix compare a stat to the second: a file rewritten to the same size in the second it
/// was staged would read as unchanged once the index is rewritten a second later.
pub(crate) fn write_index(
    repo: &git2::Repository,
    index: &mut git2::Index,
) -> Result<(), GitError> {
    // the index on disk is still the one read: its time is the second its entries were judged against
    if let Some((written, _)) = written_at(index) {
        smudge_racy(repo, index, written)?;
    }
    index.write().map_err(|e| failed("write the index", &e))
}

/// When `index`'s file was last written: its second, and the full time that tells a rewrite in that same second.
fn written_at(index: &git2::Index) -> Option<(i64, std::time::SystemTime)> {
    let time = std::fs::metadata(index.path()?).ok()?.modified().ok()?;
    let second = i64::try_from(time.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs()).ok()?;
    Some((second, time))
}

/// Run `op`, a libgit2 call that writes the index itself (a checkout, a reset, a merge, a stash), then keep the
/// racy-clean rule its write dropped, as [`write_index`] keeps it: libgit2 judged the entries against the index it
/// replaced in nanoseconds, so one racy to that index's second whose file now differs is smudged and the index written
/// again. The result is `op`'s; an index `op` did not rewrite is left alone. `op` has landed once it returns, so a
/// fix-up that fails after it is said, never answered as `op`'s failure: the port has no channel to a verb's answer,
/// and the person running the verb reads its stderr.
pub(crate) fn kept<T>(
    place: &GitPlace,
    op: impl FnOnce() -> Result<T, GitError>,
) -> Result<T, GitError> {
    let (value, unkept) = kept_with(place, op, |index| index.write())?;
    if let Some(why) = unkept {
        eprintln!("{why}");
    }
    Ok(value)
}

/// [`kept`], the fix-up's index written by `write`: `op`'s value, and when the fix-up failed once `op` landed, what
/// says so.
pub(crate) fn kept_with<T>(
    place: &GitPlace,
    op: impl FnOnce() -> Result<T, GitError>,
    write: impl FnOnce(&mut git2::Index) -> Result<(), git2::Error>,
) -> Result<(T, Option<String>), GitError> {
    // a repository this cannot open, `op` refuses on its own
    let before = crate::write::libgit2_read(place).ok().and_then(|repo| {
        let index = repo.index().ok()?;
        let written = written_at(&index)?;
        Some((repo, index, written))
    });
    let value = op()?;
    let Some((repo, mut index, (second, time))) = before else {
        return Ok((value, None));
    };
    let kept = index
        .read(true)
        .map_err(|e| failed("read the index", &e))
        .and_then(|()| match written_at(&index) {
            Some((_, now)) if now != time => {
                if smudge_racy(&repo, &mut index, second)? {
                    write(&mut index).map_err(|e| failed("write the index", &e))?;
                }
                Ok(())
            }
            _ => Ok(()),
        });
    let unkept = kept.err().map(|why| {
        format!(
            "the operation completed; only keeping git's racy-clean rule in the index failed ({}): a file rewritten in \
             this same second may read as unchanged until git refreshes the index (run git status)",
            why.0
        )
    });
    Ok((value, unkept))
}

/// Smudge (size 0) each entry of `index` stamped in or after the second `written` whose file differs from it, as git
/// does before it writes an index; `true` when one was.
fn smudge_racy(
    repo: &git2::Repository,
    index: &mut git2::Index,
    written: i64,
) -> Result<bool, GitError> {
    const STAGE_MASK: u16 = 0x3000;
    const GITLINK: u32 = 0o160000;
    let racy: Vec<Vec<u8>> = index
        .iter()
        .filter(|entry| {
            entry.flags & STAGE_MASK == 0
                && entry.mode != GITLINK
                && entry.file_size != 0
                && i64::from(entry.mtime.seconds()) >= written
        })
        .map(|entry| entry.path)
        .collect();
    if racy.is_empty() {
        return Ok(false);
    }
    let mut options = git2::DiffOptions::new();
    options
        .include_typechange(true)
        .ignore_submodules(true)
        .disable_pathspec_match(true);
    for path in racy {
        options.pathspec(path);
    }
    let changed: Vec<std::path::PathBuf> = repo
        .diff_index_to_workdir(Some(index), Some(&mut options))
        .map_err(|e| failed("compare the index with the files", &e))?
        .deltas()
        .filter_map(|delta| delta.old_file().path().map(Path::to_path_buf))
        .collect();
    let mut smudged = false;
    for path in changed {
        if let Some(mut entry) = index.get_path(&path, 0) {
            entry.file_size = 0;
            index
                .add(&entry)
                .map_err(|e| failed("write the index", &e))?;
            smudged = true;
        }
    }
    Ok(smudged)
}

#[cfg(test)]
mod tests;
