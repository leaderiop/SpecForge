//! Reading a project file (ADR 0264): only a regular file, a link followed to one, and at most
//! [`MAX_FILE_BYTES`]. A named pipe, a device, a socket or a directory is refused before it is opened (opening a
//! pipe waits for a writer; reading a device never ends); a larger file is refused before it is read. The project's
//! spec, source and manifest reads go through here (a user-named plan file or credential file does not); [`crate::fresh`] is where one is written.

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;
use std::time::Instant;

use sha2::{Digest as _, Sha256};

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

/// The first `max` bytes of `path` ([`open`]), and how long it is; nothing past `max` and a byte is read.
pub fn read_prefix(path: &Path, max: u64) -> io::Result<(Vec<u8>, u64)> {
    let file = open(path)?;
    let length = file.metadata()?.len();
    let mut bytes = Vec::new();
    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    // Linux's `/proc` reports a smaller length than it holds.
    let length = length.max(bytes.len() as u64);
    bytes.truncate(usize::try_from(max).unwrap_or(usize::MAX));
    Ok((bytes, length))
}

/// `path`, opened for reading, when it is a regular file of any size (a link is followed): what a caller that streams
/// or caps its own read opens. The handle opened is checked again.
pub fn open_any(path: &Path) -> io::Result<File> {
    judge_kind(&std::fs::metadata(path)?)?;
    let file = File::open(path)?;
    judge_kind(&file.metadata()?)?;
    Ok(file)
}

/// The SHA-256 of `path`, in lowercase hex, and how long it is: a regular file of any size, in chunks, so a digest pin
/// holds a large artifact. `TimedOut` once `deadline` passes.
pub fn digest(path: &Path, deadline: Instant) -> io::Result<(String, u64)> {
    judge_kind(&std::fs::metadata(path)?)?;
    let mut file = File::open(path)?;
    judge_kind(&file.metadata()?)?;
    let mut hasher = Sha256::new();
    let mut chunk = vec![0; 1024 * 1024];
    let mut length = 0;
    loop {
        if Instant::now() >= deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let read = file.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        hasher.update(&chunk[..read]);
        length += read as u64;
    }
    Ok((hex_of(&hasher.finalize()), length))
}

/// The SHA-256 of `bytes`, in lowercase hex: what [`digest`] gives for a file holding them.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_of(&Sha256::digest(bytes))
}

fn hex_of(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
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
    judge_kind(meta)?;
    if meta.len() > MAX_FILE_BYTES {
        Err(too_large())
    } else {
        Ok(())
    }
}

fn judge_kind(meta: &std::fs::Metadata) -> io::Result<()> {
    let kind = meta.file_type();
    if kind.is_file() {
        return Ok(());
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{digest, sha256_hex};

    #[test]
    fn a_digest_streams_the_file_and_stops_at_its_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc");
        std::fs::write(&path, "abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let later = Instant::now() + Duration::from_secs(60);
        assert_eq!(digest(&path, later).unwrap(), (abc.to_string(), 3));
        assert_eq!(sha256_hex(b"abc"), abc);

        let past = Instant::now();
        assert_eq!(
            digest(&path, past).unwrap_err().kind(),
            std::io::ErrorKind::TimedOut
        );
        assert_eq!(
            digest(dir.path(), later).unwrap_err().kind(),
            std::io::ErrorKind::IsADirectory
        );
    }
}
