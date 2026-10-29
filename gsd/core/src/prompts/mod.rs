//! The prompts of `@gsd/core`: the lifecycle workflows of GSD, ported (r42). `assets` embeds the ported text,
//! `repoint` points its references at the command that serves them, `render` builds the text a model reads for a host,
//! and each family of prompts is a table of `workflow::Spec` rows.

#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
mod args;
pub(crate) mod assets;
#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
mod host;
#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
mod preamble;
#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
pub(crate) mod render;
pub(crate) mod repoint;
#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
mod slash;
mod spec;
#[allow(
    dead_code,
    reason = "the prompts that use the renderer are declared by the tickets that follow"
)]
mod workflow;

use specforge_extension_sdk::prelude::*;

pub use spec::GATES;

pub fn declare(c: &mut ContributionsBuilder) {
    spec::declare(c);
}
