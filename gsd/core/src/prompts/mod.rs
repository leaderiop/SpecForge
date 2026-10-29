//! The prompts of `@gsd/core`: the lifecycle workflows of GSD, ported (r42). `assets` embeds the ported text,
//! `repoint` points its references at the command that serves them, `render` builds the text a model reads for a host,
//! and each family of prompts is a table of `workflow::Spec` rows.

mod args;
pub(crate) mod assets;
mod discuss;
mod host;
mod plan;
mod preamble;
mod project;
pub(crate) mod render;
pub(crate) mod repoint;
mod slash;
mod spec;
mod workflow;

use specforge_extension_sdk::prelude::*;

pub use spec::GATES;

pub fn declare(c: &mut ContributionsBuilder) {
    spec::declare(c);
    project::declare(c);
    discuss::declare(c);
    plan::declare(c);
}
