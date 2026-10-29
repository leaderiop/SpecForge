//! The passes of `@gsd/core`.

mod commits;
mod cross_plan;
mod derive;
mod drift;
mod execute;
mod fails_when;
mod file_scope;
mod gates;
mod nyquist;
mod plan_shape;
mod plans;
mod research;
mod roadmap;
mod scan;
mod schema_drift;
mod scope;
mod verify_paths;
mod verify_rules;
mod waves;
mod windows;

use specforge_extension_sdk::prelude::*;

/// The computed fields a compute pass other than `derive` answers: `derive` lists every other `C` field of the model,
/// and the host refuses a field two compute passes claim (W021). Keep this in step with the passes' `computes`.
pub const CLAIMED: &[&str] = &[
    "plan.wave",
    "plan.autonomous",
    "task.verify_paths",
    "artifact.state",
    "key_link.state",
    "verification.anti_patterns",
    "verification.scan_partial",
    "validation_row.file_exists",
    "wave0_item.done",
];

pub fn declare(c: &mut ContributionsBuilder) {
    // The scan is declared before `derive`: compute passes run in declaration order and derive reads its values.
    // `waves` computes `plan.wave`, which derive reads for a validation row.
    waves::declare(c);
    scan::declare(c);
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
    nyquist::declare(c);
    verify_rules::declare(c);
    windows::declare(c);
    gates::declare(c);
}
