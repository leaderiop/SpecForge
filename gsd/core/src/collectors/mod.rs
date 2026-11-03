//! The collectors of `@gsd/core`: what brings the outside world into the project (`spec/execute.md`).
//!
//! `gsd-git` is detected by a branch ref (`.git/refs/heads/*` or `.git/packed-refs`: a repository with a commit, ADR
//! 0331 D4), so a plain `specforge collect` records the commits; where it gives no evidence (`--no-run`, no `git`)
//! the host skips it. `gsd-junit` declares no detection: it has no command, so a plain collect could never run it.

mod git;
mod graph;
mod junit;

pub(crate) use git::LinkedBy;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    git::declare(c);
    junit::declare(c);
}
