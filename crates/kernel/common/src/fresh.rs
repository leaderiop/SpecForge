//! Writing a project file by replacing its name (ADR 0144). A path rule cannot see a hard link, so the
//! rule is on the write: the inode that held a name is never opened for writing. A file is replaced by
//! creating a new one beside it and renaming it over the name; a stream that cannot be staged is
//! created new after its name is unlinked. A link at the name or at the temporary name is removed,
//! never followed. A file of the user's registry directory is written privately (ADR 0193).

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read as _, Result, Write as _};
use std::path::{Path, PathBuf};

/// Replace `path` atomically: write `<path>.tmp` (the file name with `.tmp` appended, beside `path`),
/// synced, then rename it onto `path`. Whatever is at the temp name first (a file left by a crash, or a
/// link) is removed, a link and never its target, and the temp is created new (`O_EXCL`), so it is never
/// written through a link. On failure the temp is removed and `path` is as it was.
///
/// A file that is there keeps its permissions, and one the process may not write is refused with the
/// error of opening it for writing (`PermissionDenied`), as an in-place write would have: its owner's
/// read-only marker is not lifted by a rewrite. The temp gets them before a byte is written. Ownership and extended attributes are not carried over.
pub fn replace_file(path: &Path, contents: &[u8]) -> Result<()> {
    let permissions = match OpenOptions::new().write(true).open(path) {
        Ok(existing) => Some(existing.metadata()?.permissions()),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let temp = temporary(path);
    unlink(&temp)?;
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            if let Some(permissions) = permissions {
                file.set_permissions(permissions)?;
            }
            file.write_all(contents)?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

/// Replace `path` privately (ADR 0193): a file in the user's registry directory. Its missing parent
/// directories are created 0700 and `<path>.tmp` is created new at 0600 before a byte is written, synced,
/// then renamed onto `path` (unix; elsewhere no mode is set, ADR 0178). Whatever is at the temp name is
/// unlinked first, never followed, and what is at `path` (its mode, a link) is replaced, not kept. On
/// failure the temp is removed and `path` is as it was.
pub fn replace_private(path: &Path, contents: &[u8]) -> Result<()> {
    private_parent(path)?;
    let temp = temporary(path);
    unlink(&temp)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let written = options
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(contents)?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

/// What [`replace_private`] wrote at `path`: its bytes, when it is a file of this user that no one else may touch. A
/// link, a file of another mode (a git checkout writes 0644) or of another owner is `PermissionDenied`, so a project
/// cannot commit one in the store's place. Elsewhere than unix no mode is set, so this cannot tell, and reads it.
pub fn read_private(path: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_NOFOLLOW);
    let mut file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let meta = file.metadata()?;
        #[expect(unsafe_code, reason = "geteuid(2) takes nothing and cannot fail")]
        // SAFETY: no argument, no memory, and the call always succeeds.
        let me = unsafe { libc::geteuid() };
        if !meta.is_file() || meta.mode() & 0o077 != 0 || meta.uid() != me {
            return Err(ErrorKind::PermissionDenied.into());
        }
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Create the missing parent directories of `path`, 0700 (unix).
pub fn private_parent(path: &Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let mut dirs = std::fs::DirBuilder::new();
    dirs.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut dirs, 0o700);
    dirs.create(parent)
}

/// Open the sidecar lock file `path`, created 0600 (unix). A link at the name is refused, never followed: creating
/// through a dangling link would make a file wherever it points. Elsewhere than unix the name is looked at first.
pub fn open_private_lock(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(std::io::Error::other("it is a link"));
    }
    options.open(path)
}

/// Make the existing directory `path` readable by its owner alone (`0700`, unix) through a handle that refuses a link
/// at the name (`O_NOFOLLOW`), so a link swapped in is never followed to chmod what it points at. Elsewhere than unix
/// no mode exists, and nothing changes.
pub fn owner_only_dir(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(path)?;
        dir.set_permissions(std::fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Why the file or directory `meta` describes is not this user's alone to write: it belongs to another user, or its
/// group or others may write it. `None` when it is, and always elsewhere than unix, where no such mode exists
/// (ADR 0178). For what a private file trusts: another user could have created it first and filled it.
pub fn not_private(meta: &std::fs::Metadata) -> Option<&'static str> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        #[expect(unsafe_code, reason = "geteuid(2) takes nothing and always succeeds")]
        let me = unsafe { libc::geteuid() };
        judged_private(meta.uid(), meta.mode(), me)
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        None
    }
}

/// [`not_private`] of an `owner` and a `mode`, for the user `me`.
#[cfg(unix)]
fn judged_private(owner: u32, mode: u32, me: u32) -> Option<&'static str> {
    if owner != me {
        Some("belongs to another user")
    } else if mode & 0o022 != 0 {
        Some("is writable by its group or others")
    } else {
        None
    }
}

/// A new file at `path`, to stream into: the name is unlinked first (a link, never its target; a
/// missing name passes), then the file is created new, so the old inode is never opened. A name
/// recreated between the two steps fails with `AlreadyExists` instead of escaping.
pub fn create_fresh(path: &Path) -> Result<File> {
    unlink(path)?;
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn unlink(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// `<path>.tmp`: the file name with `.tmp` appended.
fn temporary(path: &Path) -> PathBuf {
    let mut name: OsString = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn private_means_this_users_and_no_one_elses_to_write() {
        for mode in [0o600, 0o644, 0o700, 0o755, 0o400] {
            assert_eq!(judged_private(501, mode, 501), None, "{mode:o}");
        }
        for mode in [0o620, 0o602, 0o660, 0o666, 0o770, 0o707, 0o1777] {
            assert_eq!(
                judged_private(501, mode, 501),
                Some("is writable by its group or others"),
                "{mode:o}"
            );
        }
        for (owner, me) in [(502, 501), (0, 501), (501, 0)] {
            assert_eq!(
                judged_private(owner, 0o600, me),
                Some("belongs to another user"),
                "{owner} as {me}"
            );
        }
    }

    #[test]
    fn a_file_this_user_just_made_privately_is_private() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x");
        replace_private(&path, b"x").unwrap();
        assert_eq!(not_private(&std::fs::metadata(&path).unwrap()), None);
        assert_eq!(not_private(&std::fs::metadata(dir.path()).unwrap()), None);
    }

    #[cfg(unix)]
    #[test]
    fn replace_file_leaves_a_hard_links_other_name_alone() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other");
        std::fs::write(&other, "ORIGINAL").unwrap();
        std::fs::hard_link(&other, dir.path().join("x")).unwrap();
        replace_file(&dir.path().join("x"), b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "ORIGINAL");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("x")).unwrap(),
            "new"
        );
    }

    #[cfg(unix)]
    #[test]
    fn replace_file_unlinks_a_temp_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other");
        std::fs::write(&other, "ORIGINAL").unwrap();
        std::os::unix::fs::symlink(&other, dir.path().join("x.tmp")).unwrap();
        replace_file(&dir.path().join("x"), b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "ORIGINAL");
        assert!(!dir.path().join("x.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn replace_file_refuses_a_read_only_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        // A process that may write anyway (root) cannot show the refusal.
        if OpenOptions::new().write(true).open(&path).is_ok() {
            return;
        }
        let error = replace_file(&path, b"new").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
    }

    #[cfg(unix)]
    #[test]
    fn create_fresh_never_opens_the_old_inode() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other");
        std::fs::write(&other, "ORIGINAL").unwrap();
        std::fs::hard_link(&other, dir.path().join("x")).unwrap();
        let mut file = create_fresh(&dir.path().join("x")).unwrap();
        file.write_all(b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "ORIGINAL");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("x")).unwrap(),
            "new"
        );
    }

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn replace_private_creates_its_file_0600_in_0700_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/x");
        replace_private(&path, b"new").unwrap();
        assert_eq!(mode_of(&dir.path().join("a")), 0o700);
        assert_eq!(mode_of(&dir.path().join("a/b")), 0o700);
        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!dir.path().join("a/b/x.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn replace_private_never_follows_or_keeps_a_stale_temporary() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other");
        std::fs::write(&other, "ORIGINAL").unwrap();
        let (path, temp) = (dir.path().join("x"), dir.path().join("x.tmp"));
        std::os::unix::fs::symlink(&other, &temp).unwrap();
        replace_private(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "ORIGINAL");
        assert_eq!(mode_of(&path), 0o600);
        assert!(
            !std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        std::fs::write(&temp, "stale").unwrap();
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o644)).unwrap();
        replace_private(&path, b"newer").unwrap();
        assert_eq!(mode_of(&path), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn replace_private_replaces_a_files_mode_and_a_read_only_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x");
        for old in [0o644, 0o444] {
            std::fs::write(&path, "old").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(old)).unwrap();
            replace_private(&path, b"new").unwrap();
            assert_eq!(mode_of(&path), 0o600);
            assert_eq!(std::fs::read(&path).unwrap(), b"new");
        }
    }
}
