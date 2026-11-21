//! The passes of `@gsd/core`.

mod citations;
pub(crate) mod commits;
mod cross_plan;
mod decision_coverage;
pub(crate) mod derive;
mod execute;
mod fails_when;
mod file_scope;
mod gates;
mod nyquist;
mod plan_shape;
pub(crate) mod plans;
mod probe_rows;
mod research;
mod roadmap;
mod scan;
mod schema_drift;
mod scope;
mod stale_truths;
pub(crate) mod thread;
mod traceability;
mod verify_paths;
mod verify_rules;
mod wave_collision;
mod waves;
mod windows;
pub(crate) mod words;
mod workstreams;

use specforge_extension_sdk::prelude::*;

/// The computed fields a compute pass other than `derive` answers: `derive` lists every other `C` field of the model,
/// and the host refuses a field two compute passes claim (W021). Keep this in step with the passes' `computes`.
pub const CLAIMED: &[&str] = &[
    "plan.wave",
    "plan.autonomous",
    "plan.strict_wave",
    "task.verify_paths",
    "artifact.state",
    "key_link.state",
    "verification.anti_patterns",
    "verification.scan_partial",
    "validation_row.file_exists",
    "wave0_item.done",
];

/// A text a pass echoes but did not write (a path, an id, a field or an input name), escaped so its message and its
/// help stay one line. An id or a name of the grammar comes out unchanged; the one-line text does not rest on that.
pub(crate) fn one_line(text: &str) -> String {
    text.escape_debug().to_string()
}

/// `text` [`one_line`] between single quotes: how a message or a help names an id or a path it echoes.
pub(crate) fn quoted(text: &str) -> String {
    format!("'{}'", one_line(text))
}

pub fn declare(c: &mut ContributionsBuilder) {
    // The scan is declared before `derive`: compute passes run in declaration order and derive reads its values.
    // `waves` computes `plan.wave`, which derive reads for a validation row.
    waves::declare(c);
    scan::declare(c);
    derive::declare(c);
    roadmap::declare(c);
    research::declare(c);
    execute::declare(c);
    schema_drift::declare(c);
    plan_shape::declare(c);
    probe_rows::declare(c);
    cross_plan::declare(c);
    decision_coverage::declare(c);
    fails_when::declare(c);
    file_scope::declare(c);
    wave_collision::declare(c);
    scope::declare(c);
    verify_paths::declare(c);
    nyquist::declare(c);
    verify_rules::declare(c);
    stale_truths::declare(c);
    windows::declare(c);
    workstreams::declare(c);
    gates::declare(c);
    traceability::declare(c);
}
