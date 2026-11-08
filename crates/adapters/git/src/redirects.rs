//! Where a repository's reads are led beyond its own `.git` directory: a `.git` file or link, a `commondir`, a
//! `core.worktree` and the object alternates. A caller that holds its reads to some roots asks here (`Git::leads_to`).

use std::path::{Path, PathBuf};

use specforge_effects::git::GitPlace;

/// git's own limit on how deep alternates chain.
const ALTERNATES_DEPTH: usize = 5;

/// The directories the repository at `place` reads beyond its `.git`. A real linked worktree and a real submodule name
/// their `.git` back, and only what their repository is can do that, so theirs lead nowhere of note. A repository found
/// by a walk upward is no redirect.
pub(crate) fn leads_to(place: &GitPlace) -> Vec<PathBuf> {
    let own = place.toplevel.join(".git");
    let plain = std::fs::symlink_metadata(&own).is_ok_and(|meta| meta.is_dir())
        && std::fs::canonicalize(&own).is_ok_and(|real| real == place.git_dir)
        && place.git_dir == place.common_dir;
    let mut led = Vec::new();
    if !plain && !real_worktree(place, &own) && !real_submodule(place, &own) {
        led.extend([
            place.toplevel.clone(),
            place.git_dir.clone(),
            place.common_dir.clone(),
        ]);
    }
    alternates(&place.common_dir.join("objects"), 0, &mut led);
    led.retain(|at| !at.starts_with(&place.common_dir) || at == &place.common_dir);
    led.dedup();
    led
}

/// `place`'s git directory is an admin directory of its common one (`worktrees/<id>`), whose `gitdir` file names `own`.
fn real_worktree(place: &GitPlace, own: &Path) -> bool {
    if place.git_dir.parent() != Some(place.common_dir.join("worktrees").as_path()) {
        return false;
    }
    std::fs::read_to_string(place.git_dir.join("gitdir")).is_ok_and(|text| {
        same(
            &place.git_dir.join(text.trim_end_matches(['\r', '\n'])),
            own,
        )
    })
}

/// `own` is a file naming `place`'s git directory, whose `core.worktree` names `place`'s top back.
fn real_submodule(place: &GitPlace, own: &Path) -> bool {
    if !std::fs::symlink_metadata(own).is_ok_and(|meta| meta.is_file()) {
        return false;
    }
    git2::Config::open(&place.git_dir.join("config"))
        .and_then(|config| config.get_path("core.worktree"))
        .is_ok_and(|worktree| same(&place.git_dir.join(worktree), &place.toplevel))
}

fn same(one: &Path, other: &Path) -> bool {
    match (std::fs::canonicalize(one), std::fs::canonicalize(other)) {
        (Ok(one), Ok(other)) => one == other,
        _ => false,
    }
}

/// Each alternate `objects` names, and those it names in turn. One that is not there is kept as written: it is still
/// where the repository says to read.
fn alternates(objects: &Path, depth: usize, led: &mut Vec<PathBuf>) {
    let Ok(text) = std::fs::read_to_string(objects.join("info/alternates")) else {
        return;
    };
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let written = objects.join(line);
        let at = std::fs::canonicalize(&written).unwrap_or(written);
        if led.contains(&at) {
            continue;
        }
        led.push(at.clone());
        if depth + 1 < ALTERNATES_DEPTH {
            alternates(&at, depth + 1, led);
        }
    }
}
