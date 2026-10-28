//! The passes of `@gsd/core`.

mod derive;
mod drift;
mod file_scope;
mod plan_shape;
mod plans;
mod research;
mod roadmap;
mod waves;

use specforge_extension_sdk::prelude::*;

/// The computed fields a compute pass other than `derive` answers: `derive` lists every other `C` field of the model,
/// and the host refuses a field two compute passes claim (W021). Keep this in step with the passes' `computes`.
pub const CLAIMED: &[&str] = &["plan.wave", "plan.autonomous"];

pub fn declare(c: &mut ContributionsBuilder) {
    derive::declare(c);
    roadmap::declare(c);
    research::declare(c);
    drift::declare(c);
    plan_shape::declare(c);
    file_scope::declare(c);
    waves::declare(c);
}
