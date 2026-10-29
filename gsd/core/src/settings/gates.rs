//! The rules a setting switches off while it is false ([Setting-gated rules](../../spec/rules.md#setting-gated-rules)),
//! and the four settings of `@gsd/core` GSD does not have.

use specforge_extension_sdk::prelude::*;

/// `(setting, codes)`: the setting is a GSD key, declared in `keys.rs`, that gates these local codes.
pub const GATES: &[(&str, &[&str])] = &[
    ("workflow.windows_enforce", &["E512"]),
    ("workflow.tdd_mode", &["W403", "W404"]),
    ("workflow.nyquist_validation", &["E314", "W510"]),
    (
        "workflow.plan_check",
        &["W311", "W313", "W315", "W316", "W318"],
    ),
    ("workflow.post_planning_gaps", &["W317"]),
    ("workflow.context_coverage_gate", &["E310", "W506"]),
    ("workflow.schema_drift_gate", &["E405"]),
    ("workflow.context_drift_precheck", &["W406"]),
];

/// `(setting, code, what it switches on)`: SpecForge's own, under `specforge.` so the roster tells them apart.
pub const SPECFORGE: &[(&str, &str, &str)] = &[
    (
        "specforge.task_commits",
        "W407",
        "Every task of a complete plan has a commit.",
    ),
    (
        "specforge.traceability_gate",
        "E515",
        "A complete requirement traces to a task, a commit and a passing test.",
    ),
    (
        "specforge.stale_verdict_gate",
        "E516",
        "A stale verdict blocks every workflow step, not only execution.",
    ),
    (
        "specforge.cross_workstream_conflicts",
        "E517",
        "Active plans of different workstreams share no file.",
    ),
];

/// The codes `setting` gates, if it is a GSD key that gates any.
pub fn of(setting: &str) -> Option<&'static [&'static str]> {
    GATES.iter().find(|(s, _)| *s == setting).map(|(_, c)| *c)
}

pub fn declare(c: &mut ContributionsBuilder) {
    for (name, code, about) in SPECFORGE {
        c.setting(name, |s| {
            s.bool().default(false).describe(about).gates(&[code]);
        });
    }
}
