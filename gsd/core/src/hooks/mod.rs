//! The hooks of `@gsd/core` (r44): upstream's hook scripts as typed contributions, each a pure function of the host's
//! facts, the extension's settings and its state. `roster` says what became of each of the 28 upstream files.

#[cfg(test)]
mod accounting;
mod community;
mod config;
mod context;
#[cfg(test)]
mod corpus;
mod facts;
mod guards;
mod injection;
mod isolation;
pub mod roster;
mod secrets;
mod statusline;
mod workflow;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    guards::declare(c);
    secrets::declare(c);
    injection::declare(c);
    isolation::declare(c);
    workflow::declare(c);
    context::declare(c);
    statusline::declare(c);
    community::declare(c);
    config::declare(c);
}
