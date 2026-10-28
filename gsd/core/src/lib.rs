//! `@gsd/core`: the unified GSD meta-model for SpecForge (model: `gsd/core/spec`).

pub mod codes;

use specforge_extension_sdk::prelude::*;

/// The gsd-core release this crate tracks; `[package.metadata.gsd] tracks` in `Cargo.toml` says the same.
pub const TRACKS: &str = "gsd-core@1.18.0";

#[specforge_extension_sdk::extension(
    name = "@gsd/core",
    short = "gsd",
    description = "GSD in SpecForge: project, roadmap, plans, execution, verification and their checks"
)]
struct GsdCore;

impl Contributions for GsdCore {
    fn contribute(c: &mut ContributionsBuilder) {
        c.meta.keywords.push(format!("tracks:{TRACKS}"));
        codes::declare(c);
    }
}

/// The extension's contributions, as its guest builds them per call (tests serve the same declaration).
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}

specforge_extension_sdk::component_guest!(build = build);
