//! Where a directory's repository is: git's discovery through gix, held to git's answer where gix's differs, with
//! git's `GIT_CEILING_DIRECTORIES` kept, a variable that names another repository, worktree or object store refused,
//! and a hook's temporary `GIT_INDEX_FILE` read (ADR 0352).

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
    // git looks for a repository only strictly below the nearest ceiling, and stops at the first `.git` there; gix
    // searches the ceiling itself and walks on past a `.git` it cannot use, opening what it finds (its config read,
    // its `core.worktree` followed). So the search ends where git's would: at the first candidate below the ceiling.
    let nearest = ceilings
        .iter()
        .max_by_key(|ceiling| ceiling.components().count());
    let candidate = nearest.map(|ceiling| {
        here.ancestors()
            .take_while(|at| at != ceiling)
            .find(|at| {
                at.join(".git").symlink_metadata().is_ok() || gix::discover::is_git(at).is_ok()
            })
            .map(Path::to_path_buf)
    });
    let ceiling_dirs = match &candidate {
        Some(None) => {
            return named_elsewhere(dir, None, env)
                .map_or(Located::NotARepository, Located::Unavailable);
        }
        Some(Some(at)) => vec![at.clone()],
        None => ceilings,
    };
    let upwards = gix::discover::upwards::Options {
        ceiling_dirs,
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
            let refused = named_elsewhere(dir, None, env);
            return match candidate {
                Some(Some(at)) => Located::Unavailable(refused.unwrap_or_else(|| {
                    format!("{} has a .git that is no repository", at.display())
                })),
                _ => refused.map_or(Located::NotARepository, Located::Unavailable),
            };
        }
        Err(error) => return Located::Unavailable(error.to_string()),
    };
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
        index: None,
    };
    if let Some(why) = named_elsewhere(dir, Some(&place), env) {
        return Located::Unavailable(why);
    }
    match index_file(&place, env) {
        Ok(index) => Located::Repo(GitPlace { index, ..place }),
        Err(why) => Located::Unavailable(why),
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

/// Why a variable naming another repository, worktree or object store refuses locating `dir`, found in `place` (none:
/// in no repository). git reads what `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY` and
/// `GIT_ALTERNATE_OBJECT_DIRECTORIES` name in place of, or beside, the repository around the directory; the host never
/// lets the environment point it elsewhere (ADR 0339 D5), so one naming another than `place`'s own refuses, naming
/// itself, rather than answer for a repository git would not read there. One naming `place`'s own (a hook git runs in
/// a linked worktree sets `GIT_DIR`) is kept. In no repository only `GIT_DIR` and `GIT_WORK_TREE` refuse: git reads
/// the others only of a repository it found.
fn named_elsewhere(dir: &Path, place: Option<&GitPlace>, env: Env<'_>) -> Option<String> {
    let mut own = vec![
        ("GIT_DIR", place.map(|place| place.git_dir.clone())),
        ("GIT_WORK_TREE", place.map(|place| place.toplevel.clone())),
    ];
    if let Some(place) = place {
        own.push(("GIT_COMMON_DIR", Some(place.common_dir.clone())));
        own.push((
            "GIT_OBJECT_DIRECTORY",
            Some(place.common_dir.join("objects")),
        ));
    }
    let around = match place {
        Some(place) => format!("is in the repository at {}", place.toplevel.display()),
        None => "is in no repository".to_string(),
    };
    let refusal = |name: &str, value: &Path| {
        format!(
            "{name} names {}, but {} {around}: specforge reads the repository a directory is in, never one the \
             environment names, so unset {name} or run from the repository it names",
            shown(value),
            dir.display()
        )
    };
    let named = own.into_iter().find_map(|(name, own)| {
        let value = env(name).filter(|value| !value.is_empty())?;
        (own.is_none() || real_path(&value) != own).then(|| refusal(name, Path::new(&value)))
    });
    // each alternate the environment adds is another object store, unless it is the repository's own
    let alternates = || {
        let place = place?;
        let value = env("GIT_ALTERNATE_OBJECT_DIRECTORIES")?;
        std::env::split_paths(&value)
            .filter(|entry| !entry.as_os_str().is_empty())
            .find(|entry| real_path(entry.as_os_str()) != Some(place.common_dir.join("objects")))
            .map(|entry| refusal("GIT_ALTERNATE_OBJECT_DIRECTORIES", &entry))
    };
    named.or_else(alternates)
}

/// The index `GIT_INDEX_FILE` names for `place`: none when it is unset, the file when it lies in `place`'s git
/// directory (git hands a hook a temporary index there for `commit -a` and `commit <paths>`, the one it commits), and
/// why not anywhere else, which would be another repository's index or none git keeps.
fn index_file(place: &GitPlace, env: Env<'_>) -> Result<Option<PathBuf>, String> {
    let Some(value) = env("GIT_INDEX_FILE").filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let written = std::path::absolute(&value).unwrap_or_else(|_| PathBuf::from(&value));
    let held = match (
        written
            .parent()
            .and_then(|parent| real_path(parent.as_os_str())),
        written.file_name(),
    ) {
        (Some(parent), Some(name)) if parent.starts_with(&place.git_dir) => Some(parent.join(name)),
        _ => None,
    };
    held.map(Some).ok_or_else(|| {
        format!(
            "GIT_INDEX_FILE names {}, outside the git directory of the repository at {}: specforge reads an index the \
             environment names only there (a temporary index git hands a hook), so unset GIT_INDEX_FILE",
            shown(Path::new(&value)),
            place.toplevel.display()
        )
    })
}

/// `value` as written, and where it lands when it is relative (git reads it from the working directory).
fn shown(value: &Path) -> String {
    match std::path::absolute(value) {
        Ok(absolute) if value.is_relative() => {
            format!("{} ({})", value.display(), absolute.display())
        }
        _ => value.display().to_string(),
    }
}

/// `value`, a path git reads relative to the working directory, by its real path; none when it does not exist.
fn real_path(value: &std::ffi::OsStr) -> Option<PathBuf> {
    std::path::absolute(value)
        .and_then(std::fs::canonicalize)
        .ok()
}
