//! The collectors of `@gsd/core`: what brings the outside world into the project (`spec/execute.md`).
//!
//! Neither declares `detect_files`: a GSD project names the one it wants, `specforge collect --runner gsd-git`. A
//! detected collector with no command would make a plain `specforge collect` fail for every runner (E045).

mod git;
mod graph;
mod junit;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    git::declare(c);
    junit::declare(c);
}
