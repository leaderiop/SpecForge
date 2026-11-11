//! Where a directory's repository is: git's discovery through gix, held to git's answer where gix's differs.

use std::path::Path;

use specforge_effects::git::{GitPlace, Located};

use crate::trust_map;

/// The repository `dir` is in, as the port's `locate` answers.
pub(crate) fn locate(dir: &Path) -> Located {
    if !dir.is_dir() {
        return Located::NotARepository;
    }
    use gix::discover::upwards::Error as Upwards;
    let repo = match gix::ThreadSafeRepository::discover_opts(dir, Default::default(), trust_map())
    {
        Ok(repo) => repo.to_thread_local(),
        Err(gix::discover::Error::Discover(
            Upwards::NoGitRepository { .. }
            | Upwards::NoGitRepositoryWithinCeiling { .. }
            | Upwards::NoGitRepositoryWithinFs { .. },
        )) => return Located::NotARepository,
        Err(error) => return Located::Unavailable(error.to_string()),
    };
    let Some(top) = repo.workdir() else {
        return Located::Unavailable("a bare repository has no working tree".into());
    };
    let real = |p: &Path| p.canonicalize().unwrap_or_else(|_| dir.join(p));
    // git answers no worktree from inside the git directory ("must be run in a work tree"); gix finds the
    // repository, so a hook would read a `.git/` path as the worktree's own
    let here = real(dir);
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
    Located::Repo(GitPlace {
        toplevel,
        git_dir: real(repo.git_dir()),
        common_dir: real(repo.common_dir()),
        shallow: repo.is_shallow(),
    })
}
