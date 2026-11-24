//! `@gsd/import`: `specforge gsd import` reads a GSD project's `.planning/` folder once and answers one edit plan
//! that writes `spec/gsd/`, plus a report naming every file it did not map, with a reason (ADR 0391).

pub mod command;
pub mod emit;
pub mod frontmatter;
pub mod ids;
pub mod kinds;
pub mod limits;
pub mod preflight;
pub mod read;
pub mod records;
pub mod report;
pub mod settings;
pub mod table;
pub mod xmltags;

use specforge_extension_sdk::prelude::*;

/// The gsd-core release this crate tracks; `[package.metadata.gsd] tracks` in `Cargo.toml` says the same.
pub const TRACKS: &str = concat!("gsd-core@", gsd_kit::tracked_version!());

/// The `@gsd/core` releases this extension writes for: the range `@gsd/security` declares.
pub const CORE_RANGE: &str = "^1.1.0";

/// The `@gsd/security` releases whose kinds a threat model maps to when the project loads it (ADR 0391 D11).
pub const SECURITY_RANGE: &str = "^1.0.0";

#[specforge_extension_sdk::extension(
    name = "@gsd/import",
    short = "gsd",
    description = "Import a GSD project: its .planning/ folder, read once, into one edit plan that writes spec/gsd/"
)]
struct GsdImport;

impl Contributions for GsdImport {
    fn contribute(c: &mut ContributionsBuilder) {
        c.meta.keywords.push(format!("tracks:{TRACKS}"));
        c.peer("@gsd/core", CORE_RANGE);
        c.optional_peer("@gsd/security", SECURITY_RANGE);
        command::declare(c);
    }
}

/// The extension's contributions, as its guest builds them per call (tests serve the same declaration).
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}

// Only the component exports `call`: a native test binary links `@gsd/core` too, whose guest exports the same symbol.
#[cfg(target_arch = "wasm32")]
specforge_extension_sdk::component_guest!(build = build);
