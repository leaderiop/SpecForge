//! Where a directory's repository is: git's discovery through gix, held to git's answer where gix's differs, with
//! git's `GIT_CEILING_DIRECTORIES` kept and a `GIT_DIR` or `GIT_WORK_TREE` that names another repository refused
//! (ADR 0352).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use specforge_effects::git::{GitPlace, Located};

use crate::trust_map;

/// Where `locate` reads git's variables: the process's environment, or the one a test gives.
pub(crate) type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

/// The repository `dir` is in, as the port's `locate` answers, git's variables read from the process's environment.
pub(crate) fn locate(dir: &Path) -> Located {
    locate_in(dir, &|name| std::env::var_os(name))
}

/// [`locate`], git's variables read from `env`.
pub(crate) fn locate_in(dir: &Path, env: Env<'_>) -> Located {
    if !dir.is_dir() {
        return Located::NotARepository;
    }
    use gix::discover::upwards::Error as Upwards;
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| dir.join(p));
    let here = real(dir);
    let ceilings = ceilings(&here, env);
    let upwards = gix::discover::upwards::Options {
        ceiling_dirs: ceilings.clone(),
        match_ceiling_dir_or_error: false,
        ..Default::default()
    };
    let repo = match gix::ThreadSafeRepository::discover_opts(&here, upwards, trust_map()) {
        Ok(repo) => repo.to_thread_local(),
        Err(gix::discover::Error::Discover(
            Upwards::NoGitRepository { .. }
            | Upwards::NoGitRepositoryWithinCeiling { .. }
            | Upwards::NoGitRepositoryWithinFs { .. },
        )) => {
            return named_elsewhere(dir, None, env)
                .map_or(Located::NotARepository, Located::Unavailable);
        }
        Err(error) => return Located::Unavailable(error.to_string()),
    };
    // git looks for a repository only strictly below the nearest ceiling; gix searches the ceiling itself, and a
    // repository found there may name its worktree elsewhere, so where it was found is what is judged
    let nearest = ceilings
        .iter()
        .max_by_key(|ceiling| ceiling.components().count());
    if nearest.is_some_and(|ceiling| {
        !here.ancestors().take_while(|at| at != ceiling).any(|at| {
            at.join(".git").symlink_metadata().is_ok() || gix::discover::is_git(at).is_ok()
        })
    }) {
        return named_elsewhere(dir, None, env)
            .map_or(Located::NotARepository, Located::Unavailable);
    }
    let Some(top) = repo.workdir() else {
        return Located::Unavailable("a bare repository has no working tree".into());
    };
    // git answers no worktree from inside the git directory ("must be run in a work tree"); gix finds the
    // repository, so a hook would read a `.git/` path as the worktree's own
    if here.starts_with(real(repo.git_dir())) || here.starts_with(real(repo.common_dir())) {
        return Located::Unavailable(format!(
            "{} is inside the git directory, which has no working tree",
            dir.display()
        ));
    }
    // git stops at a `.git` it cannot use; gix looks on upwards, and a broken repository would answer as the one
    // around it, read clean when that one is
    let toplevel = real(top);
    if here.starts_with(&toplevel)
        && let Some(broken) = here
            .ancestors()
            .take_while(|at| *at != toplevel)
            .find(|at| at.join(".git").symlink_metadata().is_ok())
    {
        return Located::Unavailable(format!(
            "{} has a .git that is no repository",
            broken.display()
        ));
    }
    let place = GitPlace {
        toplevel,
        git_dir: real(repo.git_dir()),
        common_dir: real(repo.common_dir()),
        shallow: repo.is_shallow(),
    };
    match named_elsewhere(dir, Some(&place), env) {
        Some(why) => Located::Unavailable(why),
        None => Located::Repo(place),
    }
}

/// git's `GIT_CEILING_DIRECTORIES` for a search from `here` (its real path), as git reads them: an absolute path each,
/// resolved through its links until an empty entry, and only those `here` is strictly under.
fn ceilings(here: &Path, env: Env<'_>) -> Vec<PathBuf> {
    let Some(value) = env("GIT_CEILING_DIRECTORIES") else {
        return Vec::new();
    };
    let mut resolve = true;
    std::env::split_paths(&value)
        .filter_map(|ceiling| {
            if ceiling.as_os_str().is_empty() {
                resolve = false;
                return None;
            }
            if ceiling.is_relative() {
                return None;
            }
            let ceiling = match resolve {
                true => ceiling.canonicalize().unwrap_or(ceiling),
                false => ceiling,
            };
            (here != ceiling && here.starts_with(&ceiling)).then_some(ceiling)
        })
        .collect()
}

/// Why `GIT_DIR` or `GIT_WORK_TREE` refuses locating `dir`, found in `place` (none: in no repository). git reads the
/// repository and the worktree they name in place of the ones around the directory; the host never lets the
/// environment point it elsewhere (ADR 0339 D5), so a variable naming another than `place`'s refuses, naming itself,
/// rather than answer for a repository git would not read there. One naming `place`'s own (a hook git runs in a linked
/// worktree sets `GIT_DIR`) is kept.
fn named_elsewhere(dir: &Path, place: Option<&GitPlace>, env: Env<'_>) -> Option<String> {
    let found = [
        ("GIT_DIR", place.map(|place| &place.git_dir)),
        ("GIT_WORK_TREE", place.map(|place| &place.toplevel)),
    ];
    found.into_iter().find_map(|(name, found)| {
        let value = env(name).filter(|value| !value.is_empty())?;
        // relative to the working directory, as git reads it
        let named = std::path::absolute(&value).and_then(std::fs::canonicalize).ok();
        if named.is_some() && named.as_ref() == found {
            return None;
        }
        let around = match place {
            Some(place) => format!("is in the repository at {}", place.toplevel.display()),
            None => "is in no repository".to_string(),
        };
        Some(format!(
            "{name} names {}, but {} {around}: specforge reads the repository a directory is in, never one the \
             environment names, so unset {name} or run from the repository it names",
            std::path::Path::new(&value).display(),
            dir.display()
        ))
    })
}
