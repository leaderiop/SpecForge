//! The prompts of `@gsd/core`: the lifecycle workflows of GSD, ported (r42). `assets` embeds the ported text,
//! `repoint` points its references at the command that serves them, and each family of prompts has its own module.

pub(crate) mod assets;
pub(crate) mod repoint;
mod spec;

use specforge_extension_sdk::prelude::*;

pub use spec::GATES;

pub fn declare(c: &mut ContributionsBuilder) {
    spec::declare(c);
}
