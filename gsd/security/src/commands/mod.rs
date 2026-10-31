//! The commands of `@gsd/security` (`specforge gsd <command>`; MCP `specforge.gsd.<id>`).

pub mod support;
pub mod threat_ids;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    threat_ids::declare(c);
}
