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

/// A disk, except that the ledger holds the bytes a plan will write: the plain file system for a plan being
/// judged ([`Self::new`]), the session's recording disk for a rename waiting to land ([`Self::over`]).
pub struct Overlay<'a> {
    path: PathBuf,
    ledger: &'a [u8],
    inner: &'a dyn Disk,
}

impl<'a> Overlay<'a> {
    pub fn new(root: &Path, ledger: &'a [u8]) -> Self {
        Self::over(&PlainDisk, root, ledger)
    }

    /// `inner` with the ledger of the project at `root` holding `ledger`.
    pub fn over(inner: &'a dyn Disk, root: &Path, ledger: &'a [u8]) -> Self {
        Overlay {
            path: root.join(LEDGER_FILE),
            ledger,
            inner,
        }
    }
}

impl Disk for Overlay<'_> {
    fn exists(&self, path: &Path) -> bool {
        path == self.path || self.inner.exists(path)
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        self.inner.list(dir)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if path == self.path {
            return Ok(self.ledger.to_vec());
        }
        self.inner.read(path)
    }

    fn read_ref(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.inner.read_ref(path)
    }
}
