//! `gsd-kit`: what `@gsd/core` and the capability extensions share (ADR gsd/0002): the kind DSL, the prompt render
//! pipeline and the attribution of a ported agent. No guest: each extension exports its own.

pub mod agent;
pub mod dsl;
pub mod prompt;
