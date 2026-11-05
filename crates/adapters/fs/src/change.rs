//! A person's changes (`specforge files mkdir`, `copy`, `move`, `remove`, `temp`, ADR 0339) and a file's digest
//! (`specforge files digest`): every path is held by [`Contained`] in its root, never in the host's own state, never a
//! withheld name, and a link is neither followed out of its root nor copied. A change never names a root itself, nor a
//! directory that holds one.

use std::path::{Path, PathBuf};

use specforge_effects::contained::in_host_state;
use specforge_effects::fs::LIST_VISITS;
use specforge_effects::{
    Contained, FsChange, FsChanged, FsDigest, OutcomeClass, Place, RealFs, Resolved, Root, Roots,
};

/// `place` held for a change: `Err` is the refusal.
fn placed(roots: &Roots, place: &Place) -> Result<Resolved, FsChanged> {
    let refuse = |class, detail: &str| FsChanged::of(class, format!("{}: {detail}", place.path));
    let name = Root::parse(&place.root).ok_or_else(|| refuse(OutcomeClass::Outside, "no root"))?;
    if matches!(place.path.trim_matches('/'), "" | ".") {
        return Err(refuse(
            OutcomeClass::Denied,
            "a root itself is never changed",
        ));
    }
    let resolved = Contained::new(roots, &RealFs)
        .resolve(&place.path, name)
        .map_err(|_| refuse(OutcomeClass::Outside, "it leads out of its root"))?;
    if resolved.withheld {
        return Err(refuse(
            OutcomeClass::Denied,
            "the host's own state and credential files are withheld",
        ));
    }
    if resolved.rel.is_empty() {
        return Err(refuse(
            OutcomeClass::Denied,
            "a root itself is never changed",
        ));
    }
    // A directory that holds a root, or is one: removing or replacing it would change the root.
    let real = resolved.real.clone().unwrap_or_else(|| resolved.at.clone());
    let mut every = vec![roots.project().to_path_buf()];
    every.extend(roots.get(&Root::Spec).map(Path::to_path_buf));
    every.extend(roots.named().map(|(_, at)| at.to_path_buf()));
    if every
        .iter()
        .any(|root| RealFs::landing(root).starts_with(&real))
    {
        return Err(refuse(OutcomeClass::Denied, "it is, or holds, a root"));
    }
    Ok(resolved)
}

/// Whether `path` exists, as itself (a link is not followed).
fn present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// [`specforge_effects::Fs::change`] over the local file system.
pub(crate) fn change(roots: &Roots, change: &FsChange) -> FsChanged {
    match apply(roots, change) {
        Ok(changed) => FsChanged {
            class: OutcomeClass::Ok,
            changed,
            detail: None,
        },
        Err(refused) => refused,
    }
}

fn apply(roots: &Roots, change: &FsChange) -> Result<Vec<Place>, FsChanged> {
    let failed = |error: std::io::Error, place: &Place| {
        let class = if error.kind() == std::io::ErrorKind::NotFound {
            OutcomeClass::Missing
        } else {
            OutcomeClass::Denied
        };
        FsChanged::of(class, format!("{}: {error}", place.path))
    };
    match change {
        FsChange::Mkdir { at } => {
            let held = placed(roots, at)?;
            std::fs::create_dir_all(&held.at).map_err(|e| failed(e, at))?;
            Ok(vec![named(at, &held)])
        }
        FsChange::Create { at, dir } => {
            let held = placed(roots, at)?;
            if present(&held.at) {
                return Err(FsChanged::of(
                    OutcomeClass::Denied,
                    format!("{}: it exists", at.path),
                ));
            }
            if let Some(parent) = held.at.parent() {
                std::fs::create_dir_all(parent).map_err(|e| failed(e, at))?;
            }
            private(&held.at, *dir).map_err(|e| failed(e, at))?;
            Ok(vec![named(at, &held)])
        }
        FsChange::Copy { from, to } => {
            let (source, target) = pair(roots, from, to)?;
            let mut budget = LIST_VISITS;
            copy(&source.at, &target, &mut budget).map_err(|e| failed(e, from))?;
            Ok(vec![landed(to, &target, &source, roots)])
        }
        FsChange::Move { from, to } => {
            let (source, target) = pair(roots, from, to)?;
            match std::fs::rename(&source.at, &target) {
                Ok(()) => {}
                // Another device: copied, then removed.
                Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {
                    let mut budget = LIST_VISITS;
                    copy(&source.at, &target, &mut budget).map_err(|e| failed(e, from))?;
                    remove(&source.at, true).map_err(|e| failed(e, from))?;
                }
                Err(error) => return Err(failed(error, from)),
            }
            Ok(vec![
                named(from, &source),
                landed(to, &target, &source, roots),
            ])
        }
        FsChange::Remove { at, recursive } => {
            let held = placed(roots, at)?;
            if !present(&held.at) {
                return Err(FsChanged::of(
                    OutcomeClass::Missing,
                    format!("{}: it does not exist", at.path),
                ));
            }
            remove(&held.at, *recursive).map_err(|e| failed(e, at))?;
            Ok(vec![named(at, &held)])
        }
    }
}

/// `from` (which exists) and where it goes: `to`, or into `to` when that is a directory. Never into itself.
fn pair(roots: &Roots, from: &Place, to: &Place) -> Result<(Resolved, PathBuf), FsChanged> {
    let source = placed(roots, from)?;
    if !present(&source.at) {
        return Err(FsChanged::of(
            OutcomeClass::Missing,
            format!("{}: it does not exist", from.path),
        ));
    }
    let target = placed(roots, to)?;
    let into = if target.at.is_dir() {
        let name = source.at.file_name().unwrap_or_default();
        let inside = Place {
            root: to.root.clone(),
            path: format!("{}/{}", target.rel, name.to_string_lossy()),
        };
        placed(roots, &inside)?.at
    } else {
        target.at
    };
    let real = |path: &Path| RealFs::landing(path);
    if real(&into).starts_with(real(&source.at)) {
        return Err(FsChanged::of(
            OutcomeClass::Denied,
            format!("{}: it would go into itself", to.path),
        ));
    }
    Ok((source, into))
}

/// `place` as the change answers it.
fn named(place: &Place, held: &Resolved) -> Place {
    Place {
        root: place.root.clone(),
        path: held.rel.clone(),
    }
}

/// Where a copy or move landed: `to`, or the source's name inside it.
fn landed(to: &Place, target: &Path, source: &Resolved, roots: &Roots) -> Place {
    let base = Root::parse(&to.root)
        .and_then(|root| roots.get(&root).map(Path::to_path_buf))
        .unwrap_or_default();
    let rel = target
        .strip_prefix(&base)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| source.rel.clone());
    Place {
        root: to.root.clone(),
        path: rel,
    }
}

/// A new empty file, or directory, only its owner may read (`0600`, `0700`).
fn private(at: &Path, dir: bool) -> std::io::Result<()> {
    if dir {
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(at)
    } else {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(at).map(drop)
    }
}

/// `from` copied onto `to`: a file, or a directory and what it holds, at most `budget` entries; a link, the host's own
/// state and a withheld name are left out. Nothing is written through a link: a destination that holds one (a
/// committed `b/a/x -> ~/.bashrc`) is refused, a directory is made only where none is, and a file is created new
/// after its name is unlinked, so the inode a name held is never opened (ADR 0144).
fn copy(from: &Path, to: &Path, budget: &mut usize) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(from)?;
    if !meta.is_file() && !meta.is_dir() {
        return Ok(());
    }
    match std::fs::symlink_metadata(to) {
        Ok(there) if there.file_type().is_symlink() => {
            return Err(std::io::Error::other(format!(
                "{} is a link, never written through",
                to.display()
            )));
        }
        Ok(there) if there.is_dir() != meta.is_dir() => {
            return Err(std::io::Error::other(format!(
                "{} is already there as another kind",
                to.display()
            )));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    if meta.is_file() {
        let mut source = std::fs::File::open(from)?;
        let mut target = specforge_common::fresh::create_fresh(to)?;
        std::io::copy(&mut source, &mut target)?;
        return target.set_permissions(meta.permissions());
    }
    // `mkdir` never follows a link at the name: one made since the look above fails it.
    if !present(to) {
        std::fs::create_dir(to)?;
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        *budget = budget.checked_sub(1).ok_or_else(|| {
            std::io::Error::other(format!("more than {LIST_VISITS} entries to copy"))
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if in_host_state(&name) || specforge_effects::contained::withheld(&name) {
            continue;
        }
        copy(&entry.path(), &to.join(&name), budget)?;
    }
    Ok(())
}

/// `at` removed: a file or link, an empty directory, or with `recursive` a directory and what it holds (a link inside
/// is removed, never followed).
fn remove(at: &Path, recursive: bool) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(at)?;
    match (meta.is_dir(), recursive) {
        (false, _) => std::fs::remove_file(at),
        (true, false) => std::fs::remove_dir(at),
        (true, true) => std::fs::remove_dir_all(at),
    }
}

/// [`specforge_effects::Fs::digest`] over the local file system: a regular file's SHA-256, streamed, never a withheld
/// one.
pub(crate) fn digest(roots: &Roots, at: &Place) -> FsDigest {
    let Some(name) = Root::parse(&at.root) else {
        return FsDigest::of(OutcomeClass::Outside);
    };
    let held = match Contained::new(roots, &RealFs).resolve(&at.path, name) {
        Ok(held) if held.withheld => return FsDigest::of(OutcomeClass::Denied),
        Ok(held) if held.exists && held.case.is_none() => held,
        Ok(_) => return FsDigest::of(OutcomeClass::Missing),
        Err(_) => return FsDigest::of(OutcomeClass::Outside),
    };
    // the kind is judged before the open: a named pipe would wait for a writer (ADR 0264)
    match specforge_common::regular::digest(&held.at, None) {
        Ok((sha256, bytes)) => FsDigest {
            class: OutcomeClass::Ok,
            sha256,
            bytes,
        },
        Err(_) => FsDigest::of(OutcomeClass::Denied),
    }
}
