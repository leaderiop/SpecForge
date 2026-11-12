//! The collectors of `@gsd/core`: what brings the outside world into the project (`spec/execute.md`).
//!
//! `gsd-git` is detected in any repository the project is in, at its top or below, a linked worktree's included
//! (ADR 0352 D4), so a plain `specforge collect` records the commits; it reads the host's git port, so neither
//! `--no-run` nor a missing `git` skips it.
//!
//! Tests are not GSD's to collect: a runner pack (`@javascript/node-test`, `@rust/test`) records them as
//! `@code/core`'s test facts, and a `[gsd:<id>]` tag in a test's name names its entity.

mod git;
mod graph;

pub(crate) use git::LinkedBy;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    git::declare(c);
}
