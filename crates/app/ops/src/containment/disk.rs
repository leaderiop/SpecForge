//! A containment as the `Disk` port (ADR 0139): what a crate below app reads a project file
//! through when the surface bounds it. Every read is held first, so the file opened is the
//! file checked (ADR 0104, ADR 0112 D1).

use std::io;
use std::path::{Path, PathBuf};

use specforge_registry::{Disk, PlainDisk};

use super::{Containment, ReadFailure};

/// The disk inside one containment: a name that lands outside reads as an error of kind
/// `InvalidInput` (the only way the port's `io::Result` can carry `Outside`), exists not and
/// lists nothing; anything inside is the file system's answer.
pub struct ContainedDisk(pub(crate) Containment);

impl Disk for ContainedDisk {
    fn exists(&self, path: &Path) -> bool {
        self.0
            .resolve(path)
            .is_ok_and(|held| PlainDisk.exists(&held.real))
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        PlainDisk.list(&self.0.resolve(dir).ok()?.real)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        match self.0.read(path) {
            Ok((_, bytes)) => Ok(bytes),
            Err(ReadFailure::Io { error, .. }) => Err(error),
            Err(ReadFailure::Outside(outside)) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "it resolves to {}, outside {}, which bounds the request",
                    outside.landed().display(),
                    outside.root().display()
                ),
            )),
        }
    }
}

/// The plain file system, each file read through one containment: what a declared file read (`Input::files`, ADR 0293)
/// of a call or an analyze pass reads through. It lists, tells a directory and resolves a link as the plain disk does,
/// so the shared walk still sees a linked directory and the path check still sees where a link lands; a read is the
/// containment's, so a bounded one refuses a file with several names (ADR 0146), as a report read does (ADR 0390 D12).
pub struct FileReadsDisk(pub(crate) ContainedDisk);

impl Disk for FileReadsDisk {
    fn exists(&self, path: &Path) -> bool {
        PlainDisk.exists(path)
    }

    fn list(&self, dir: &Path) -> Option<Vec<String>> {
        PlainDisk.list(dir)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.0.read(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        PlainDisk.is_dir(path)
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        PlainDisk.canonical(path)
    }
}
