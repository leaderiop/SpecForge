//! `git repos <dir>`: the repositories directly in a directory, each as the verbs would find it. A prompt asks here
//! rather than looping over a glob, and whatever cannot be read reads as neither clean nor anything else.

use std::path::Path;

use serde::Serialize;
use specforge_effects::git::Located;
use specforge_effects::{FsOp, RealFs};

use super::Verbs;
use crate::verbs::{Access, slashed};
use crate::{OpError, OpErrorKind};
use specforge_common::remedy::printable;

/// One repository directly in the directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepoEntry {
    pub name: String,
    pub path: String,
    /// Nothing to commit. False when `unreadable`: removing it could lose what could not be read.
    pub clean: bool,
    /// A linked worktree.
    pub linked: bool,
    /// Its `.git` is broken, or leads out of every root.
    pub unreadable: bool,
}

/// `git repos`'s answer, sorted by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReposAnswer {
    pub repos: Vec<RepoEntry>,
}

pub fn repos(verbs: &Verbs, dir: &Path) -> Result<ReposAnswer, OpError> {
    verbs.permit_git("repos")?;
    let at = verbs.located(dir, &Access::Fs(FsOp::Stat))?.at;
    let entries = std::fs::read_dir(&at).map_err(|e| {
        OpError::new(
            OpErrorKind::FileNotFound,
            OpErrorKind::FileNotFound.as_str(),
            format!(
                "{} cannot be listed ({e})",
                printable(&dir.to_string_lossy())
            ),
        )
    })?;
    let mut repos: Vec<RepoEntry> = entries
        .filter_map(Result::ok)
        .filter(|entry| std::fs::symlink_metadata(entry.path().join(".git")).is_ok())
        .map(|entry| {
            let child = entry.path();
            let (clean, linked) = read(verbs, &child).map_or((false, None), |(c, l)| (c, Some(l)));
            RepoEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: slashed(&child),
                clean,
                unreadable: linked.is_none(),
                linked: linked.unwrap_or(false),
            }
        })
        .collect();
    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ReposAnswer { repos })
}

/// Whether the repository at `child` is clean and linked; none when no verb could read it there.
fn read(verbs: &Verbs, child: &Path) -> Option<(bool, bool)> {
    let held = verbs.located(child, &Access::Fs(FsOp::Stat)).ok()?.at;
    let Located::Repo(place) = verbs.ports().git.locate(&held) else {
        return None;
    };
    let place = verbs.within_reach(place).ok()?;
    if place.toplevel != RealFs::landing(&held) {
        return None;
    }
    let status = verbs.ports().git.status(&place).ok()?;
    Some((!status.dirty(), place.git_dir != place.common_dir))
}
