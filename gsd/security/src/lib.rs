//! `@gsd/security`: GSD's security capability for SpecForge (plan 16): the threat model as kinds, the security audit,
//! `/gsd-secure-phase` and the ship gate. A capability extension of `@gsd/core` (ADR gsd/0002).

pub mod agents;
pub mod codes;
pub mod commands;
pub mod kinds;
mod passes;
pub mod prompts;
pub mod settings;

use specforge_extension_sdk::prelude::*;

/// The gsd-core release this crate tracks; `[package.metadata.gsd] tracks` in `Cargo.toml` says the same.
pub const TRACKS: &str = concat!("gsd-core@", gsd_kit::tracked_version!());

/// The `@gsd/core` releases this extension works with: 1.1.0 is the first whose workflows ask for hooks.
pub const CORE_RANGE: &str = "^1.1.0";

#[specforge_extension_sdk::extension(
    name = "@gsd/security",
    short = "gsd",
    description = "GSD's security capability: threat models, the security audit and the ship gate"
)]
struct GsdSecurity;

impl Contributions for GsdSecurity {
    fn contribute(c: &mut ContributionsBuilder) {
        c.meta.keywords.push(format!("tracks:{TRACKS}"));
        c.peer("@gsd/core", CORE_RANGE);
        settings::declare(c);
        codes::declare(c);
        commands::declare(c);
        kinds::declare(c);
        passes::declare(c);
        agents::declare(c);
        prompts::declare(c);
    }
}

/// The extension's contributions, as its guest builds them per call (tests serve the same declaration).
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}

// Only the component exports `call`: a native test binary links `@gsd/core` too, whose guest exports the same symbol.
#[cfg(target_arch = "wasm32")]
specforge_extension_sdk::component_guest!(build = build);
