//! Writing the ledger (ADR 0287): the lock every host write takes, and the overlay a plan is judged over.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use specforge_registry::{Disk, PlainDisk};

use super::LEDGER_FILE;

/// The lock every host write of a project takes, in the git-ignored `.specforge/`.
pub const LOCK_FILE: &str = ".specforge/write.lock";

/// An exclusive advisory lock on the project's writes, held until the file drops. A plan re-reads what it rewrites
/// under it, so two writers of disjoint files both land their lines.
pub fn lock(root: &Path) -> io::Result<File> {
    let path = root.join(LOCK_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

/// The plain file system, except that the ledger holds the bytes a plan will write.
pub struct Overlay<'a> {
    path: PathBuf,
    ledger: &'a [u8],
}

impl<'a> Overlay<'a> {
    pub fn new(root: &Path, ledger: &'a [u8]) -> Self {
        Overlay {
            path: root.join(LEDGER_FILE),
            ledger,
        }
    }
}

impl Disk for Overlay<'_> {
    fn exists(&self, path: &Path) -> bool {
        path == self.path || PlainDisk.exists(path)
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        PlainDisk.list(dir)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if path == self.path {
            return Ok(self.ledger.to_vec());
        }
        PlainDisk.read(path)
    }
}
