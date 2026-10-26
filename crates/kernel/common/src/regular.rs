//! Reading a project file (ADR 0264): only a regular file, a link followed to one, and at most
//! [`MAX_FILE_BYTES`]. A named pipe, a device, a socket or a directory is refused before it is opened (opening a
//! pipe waits for a writer; reading a device never ends); a larger file is refused before it is read. Every project
//! file SpecForge reads is read here; [`crate::fresh`] is where one is written.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

/// The most bytes read from one project file: the largest package a registry stores
/// ([`specforge_package::MAX_PACKAGE_BYTES`], 64 MiB), so every module a registry serves reads, and every other
/// project file (a source, the config, the lock) is far below it.
pub const MAX_FILE_BYTES: u64 = specforge_package::MAX_PACKAGE_BYTES;

/// `path`, opened for reading, when it is a regular file of at most [`MAX_FILE_BYTES`] (a link is followed).
/// `NotFound` when nothing is there (a dangling link too); `IsADirectory` for a directory; `FileTooLarge` over the
/// ceiling; any other type is an error naming it. The handle opened is checked again, so the file read is the file
/// judged.
pub fn open(path: &Path) -> io::Result<File> {
    judge(&std::fs::metadata(path)?)?;
    // shortcut: a pipe swapped in between the stat and the open still blocks the open (only a local process can
    // race it; a cloned repository cannot); upgrade to O_NONBLOCK when the workspace takes a libc dependency.
    let file = File::open(path)?;
    judge(&file.metadata()?)?;
    Ok(file)
}

/// The bytes of `file` (one [`open`] gave), at most [`MAX_FILE_BYTES`]: a file that grew past it, or that reports a
/// smaller length than it holds (Linux's `/proc`), is `FileTooLarge` once byte `MAX_FILE_BYTES + 1` is read.
pub fn read_from(file: File) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(too_large());
    }
    Ok(bytes)
}

/// [`open`], then [`read_from`]: the one read of a project file.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    read_from(open(path)?)
}

/// [`read`] as text: bytes that are not UTF-8 are `InvalidData`, as `std::fs::read_to_string` says.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })
}

fn judge(meta: &std::fs::Metadata) -> io::Result<()> {
    let kind = meta.file_type();
    if kind.is_file() {
        return if meta.len() > MAX_FILE_BYTES {
            Err(too_large())
        } else {
            Ok(())
        };
    }
    if kind.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::IsADirectory,
            "it is a directory, not a regular file",
        ));
    }
    Err(io::Error::other(format!(
        "it is {}, not a regular file",
        what(&kind)
    )))
}

#[cfg(unix)]
fn what(kind: &std::fs::FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt as _;
    if kind.is_fifo() {
        "a named pipe (FIFO)"
    } else if kind.is_char_device() {
        "a character device"
    } else if kind.is_block_device() {
        "a block device"
    } else if kind.is_socket() {
        "a socket"
    } else {
        "of a kind SpecForge does not read"
    }
}

#[cfg(not(unix))]
fn what(_: &std::fs::FileType) -> &'static str {
    "of a kind SpecForge does not read"
}

fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::FileTooLarge,
        format!(
            "it is larger than 64 MiB ({MAX_FILE_BYTES} bytes), the most SpecForge reads from one project file"
        ),
    )
}
