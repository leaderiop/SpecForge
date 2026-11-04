//! The debug objects macOS builds leave behind. With `split-debuginfo = "unpacked"` (its default) rustc keeps each
//! codegen unit's `.rcgu.o` beside the rlib, under a name no later build repeats, so none is ever replaced
//! (rust-lang/rust#161824): they were 25 of the 36 GB of a target. Cargo does not track them, so builds stay fresh and
//! binaries run without them; only a backtrace from an older binary loses its line numbers.

use std::fs;
use std::path::Path;
use std::time::Duration;

/// Objects younger than this may belong to a build still running.
pub const KEEP_RECENT: Duration = Duration::from_secs(60 * 60);

/// Removes the `.rcgu.o` files directly in `deps` older than `keep`, or only counts them when `dry`. The bytes freed.
pub fn prune(deps: &Path, keep: Duration, dry: bool) -> u64 {
    let mut freed = 0;
    for entry in fs::read_dir(deps).into_iter().flatten().flatten() {
        if !entry.file_name().to_string_lossy().ends_with(".rcgu.o") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let old = meta
            .modified()
            .and_then(|m| m.elapsed().map_err(std::io::Error::other))
            .is_ok_and(|age| age >= keep);
        if meta.is_file() && old && (dry || fs::remove_file(entry.path()).is_ok()) {
            freed += meta.len();
        }
    }
    freed
}

#[cfg(test)]
mod tests;
