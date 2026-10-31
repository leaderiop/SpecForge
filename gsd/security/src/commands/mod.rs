//! The commands of `@gsd/security` (`specforge gsd <command>`; MCP `specforge.gsd.<id>`).

pub mod accept;
pub mod gate;
pub mod hooks;
pub mod record;
pub mod support;
pub mod threat_ids;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    accept::declare(c);
    gate::declare(c);
    hooks::declare(c);
    record::declare(c);
    threat_ids::declare(c);
}
