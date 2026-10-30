//! The hooks of `@gsd/core` (r44): upstream's hook scripts as typed contributions, each a pure function of the host's
//! facts, the extension's settings and its state. `roster` says what became of each of the 28 upstream files.

#[cfg(test)]
mod accounting;
#[cfg(test)]
mod corpus;
mod facts;
mod guards;
mod injection;
pub mod roster;
mod secrets;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    guards::declare(c);
    secrets::declare(c);
    injection::declare(c);
}
