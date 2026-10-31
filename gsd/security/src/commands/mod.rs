//! The commands of `@gsd/security` (`specforge gsd <command>`; MCP `specforge.gsd.<id>`).

pub mod hooks;
pub mod support;
pub mod threat_ids;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    hooks::declare(c);
    threat_ids::declare(c);
}
