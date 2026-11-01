//! The plan 05 re-review (2026-11-01): the body lines (0-based) of the workflows rewritten where a line that names no
//! ported tool still told the agent something false about SpecForge's answers. `port_workflows` reads them as more
//! `REWRITTEN` rows.

pub const LINES: &[(&str, usize)] = &[
    // Plan 05 re-review item 4: a phase's counts are the progress report of that phase alone, by its id.
    ("gsd-core/workflows/execute-plan.md", 575),
    ("gsd-core/workflows/execute-plan.md", 576),
    ("gsd-core/workflows/execute-plan.md", 577),
    ("gsd-core/workflows/plan-review-convergence.md", 186),
    ("gsd-core/workflows/plan-review-convergence.md", 187),
    ("gsd-core/workflows/progress.md", 248),
    ("gsd-core/workflows/progress.md", 249),
    // Plan 05 re-review item 6: a failed plan-coverage check is an incomplete report; the argument limit is named.
    ("gsd-core/references/specless-probe-fallback.md", 98),
    ("gsd-core/workflows/review.md", 696),
    ("gsd-core/workflows/spec-phase.md", 280),
    // Plan 05 re-review item 7: progress's debt step describes verification-debt's answer; parse gaps are check's.
    ("gsd-core/workflows/progress.md", 275),
    ("gsd-core/workflows/progress.md", 279),
    ("gsd-core/workflows/progress.md", 280),
    ("gsd-core/workflows/progress.md", 282),
    ("gsd-core/workflows/progress.md", 283),
    ("gsd-core/workflows/progress.md", 293),
    ("gsd-core/workflows/progress.md", 297),
    ("gsd-core/workflows/progress.md", 299),
    ("gsd-core/workflows/progress.md", 315),
    // Plan 05 re-review item 8: the review and the learnings read the phase's own plans.
    ("gsd-core/workflows/review.md", 635),
    ("gsd-core/workflows/review.md", 634),
];
