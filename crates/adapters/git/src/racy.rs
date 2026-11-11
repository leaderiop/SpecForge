//! git's racy-clean rule kept across the index writes libgit2 makes: libgit2 judges an entry racy in nanoseconds, but
//! git and gix compare a file's stat with its entry's to the second, so an entry libgit2 leaves unsmudged can read clean
//! after a same-size rewrite in the second it was stamped.

use std::path::Path;

use specforge_effects::git::GitError;

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
    let written = index
        .path()
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|since| i64::try_from(since.as_secs()).ok());
    if let Some(written) = written {
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
        if !racy.is_empty() {
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
            for path in changed {
                if let Some(mut entry) = index.get_path(&path, 0) {
                    entry.file_size = 0;
                    index
                        .add(&entry)
                        .map_err(|e| failed("write the index", &e))?;
                }
            }
        }
    }
    index.write().map_err(|e| failed("write the index", &e))
}
