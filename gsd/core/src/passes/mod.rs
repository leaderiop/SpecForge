//! The passes of `@gsd/core`.

mod commits;
mod cross_plan;
mod derive;
mod drift;
mod execute;
mod fails_when;
mod file_scope;
mod plan_shape;
mod plans;
mod research;
mod roadmap;
mod schema_drift;
mod scope;
mod verify_paths;
mod waves;

use specforge_extension_sdk::prelude::*;

/// The computed fields a compute pass other than `derive` answers: `derive` lists every other `C` field of the model,
/// and the host refuses a field two compute passes claim (W021). Keep this in step with the passes' `computes`.
pub const CLAIMED: &[&str] = &["plan.wave", "plan.autonomous", "task.verify_paths"];

pub fn declare(c: &mut ContributionsBuilder) {
    derive::declare(c);
    roadmap::declare(c);
    research::declare(c);
    drift::declare(c);
    execute::declare(c);
    schema_drift::declare(c);
    plan_shape::declare(c);
    cross_plan::declare(c);
    fails_when::declare(c);
    file_scope::declare(c);
    scope::declare(c);
    verify_paths::declare(c);
    waves::declare(c);
}
