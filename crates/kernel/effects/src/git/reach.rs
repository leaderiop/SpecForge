//! The one containment of a located repository: where its reads lead beyond its own `.git`, held to a caller's roots.

use std::path::{Path, PathBuf};

use super::{Git, GitPlace, Located};

/// A repository that reads, beyond its own `.git`, a directory in none of the roots a caller holds: `at`, the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedOutside {
    pub toplevel: PathBuf,
    pub at: PathBuf,
}

impl std::fmt::Display for LedOutside {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the repository at {} reads {}, through its .git, a commondir, core.worktree or an alternate, and that \
             is in no root",
            self.toplevel.display(),
            self.at.display()
        )
    }
}

/// `place`, when nothing it reads beyond its own `.git` ([`Git::leads_to`]) lies outside every one of `roots` (real
/// paths, as [`crate::contained::Roots::landed`] gives them). The one containment of a located repository: a planted
/// `.git` file or link, a `commondir`, a `core.worktree` or an alternate is a redirect, and holding the directory a
/// repository was found from says nothing of where it leads. A repository around the directory is found by a walk, and
/// a real linked worktree or submodule names its `.git` back: neither is judged.
pub fn within_roots(
    git: &dyn Git,
    place: GitPlace,
    roots: &[PathBuf],
) -> Result<GitPlace, LedOutside> {
    let out = git
        .leads_to(&place)
        .into_iter()
        .find(|at| !roots.iter().any(|root| at.starts_with(root)));
    match out {
        None => Ok(place),
        Some(at) => Err(LedOutside {
            toplevel: place.toplevel,
            at,
        }),
    }
}

/// [`Git::locate`] of `dir`, a repository found held to `roots` by [`within_roots`]: one that leads out is
/// `Unavailable`, saying where.
pub fn locate_within(git: &dyn Git, dir: &Path, roots: &[PathBuf]) -> Located {
    match git.locate(dir) {
        Located::Repo(place) => match within_roots(git, place, roots) {
            Ok(place) => Located::Repo(place),
            Err(led) => Located::Unavailable(led.to_string()),
        },
        other => other,
    }
}
