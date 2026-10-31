//! `@gsd/security`: GSD's security capability for SpecForge (plan 16): the threat model as kinds, the security audit,
//! `/gsd-secure-phase` and the ship gate. A capability extension of `@gsd/core` (ADR gsd/0002).

pub mod codes;
pub mod settings;

use specforge_extension_sdk::prelude::*;

/// The gsd-core release this crate tracks; `[package.metadata.gsd] tracks` in `Cargo.toml` says the same.
pub const TRACKS: &str = "gsd-core@1.18.0";

/// The `@gsd/core` releases this extension works with. shortcut: `>=1.0.0` until core 1.1.0 (plan 16 T8) is the
/// release whose workflows ask for hooks; T8 sets `^1.1.0`.
pub const CORE_RANGE: &str = ">=1.0.0";

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
    }
}

/// The extension's contributions, as its guest builds them per call (tests serve the same declaration).
pub fn build() -> ContributionsBuilder {
    specforge_extension_build()
}

specforge_extension_sdk::component_guest!(build = build);
