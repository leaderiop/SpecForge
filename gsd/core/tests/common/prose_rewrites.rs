//! Lines (`(path, first, last)`, inclusive, 0-based body lines) rewritten by ADR 0343 D7–D9 although upstream's names
//! no tool the port rule sees: display text and value lists that read as runs, upstream verbs the port has no command
//! for, routing that probed PATH, a person's chores, and steps that read a key or a file nothing here writes.
//! `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    // the sub-repos are `specforge git repos`' children; a sub-repository is a SpecForge project of its own
    ("gsd-core/workflows/new-project.md", 653, 653),
    ("gsd-core/workflows/new-project.md", 656, 657),
    ("gsd-core/workflows/new-project.md", 669, 669),
    ("gsd-core/workflows/new-project.md", 671, 673),
    ("gsd-core/workflows/new-project.md", 675, 675),
    // display text a reader shows is quoted prose, and a list of banner names is a text fence
    (
        "gsd-core/references/autonomous-ui-design-contract.md",
        41,
        41,
    ),
    ("gsd-core/references/tdd.md", 344, 344),
    ("gsd-core/references/ui-brand.md", 50, 61),
    ("gsd-core/workflows/autonomous.md", 249, 249),
    ("gsd-core/workflows/autonomous.md", 461, 461),
    ("gsd-core/workflows/autonomous.md", 469, 469),
    ("gsd-core/workflows/autonomous.md", 483, 483),
    ("gsd-core/workflows/autonomous.md", 522, 522),
    ("gsd-core/workflows/autonomous.md", 537, 537),
    ("gsd-core/workflows/autonomous.md", 782, 782),
    ("gsd-core/workflows/autonomous.md", 784, 784),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 90, 90),
    (
        "gsd-core/workflows/execute-phase/steps/wave-post-gate-hooks.md",
        39,
        39,
    ),
    ("gsd-core/workflows/mvp-phase.md", 169, 169),
    (
        "gsd-core/workflows/plan-phase/steps/research-only-modifiers.md",
        6,
        6,
    ),
    ("gsd-core/workflows/quick/steps/discussion-phase.md", 71, 71),
    (
        "gsd-core/workflows/quick/steps/discussion-phase.md",
        121,
        121,
    ),
    (
        "gsd-core/workflows/quick/steps/plan-checker-loop.md",
        93,
        93,
    ),
    (
        "gsd-core/workflows/quick/steps/worktree-pre-dispatch-commit.md",
        2,
        2,
    ),
    ("gsd-core/workflows/ship.md", 216, 216),
    ("gsd-core/workflows/verify-work.md", 813, 813),
    // a value list names each value, and an expression is words
    ("gsd-core/references/planning-config.md", 315, 315),
    ("gsd-core/templates/summary.md", 187, 187),
    ("gsd-core/templates/summary.md", 189, 189),
    ("gsd-core/workflows/discuss-phase-power.md", 106, 106),
    ("gsd-core/workflows/onboard.md", 80, 80),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        2,
        2,
    ),
    // an upstream verb is the declared command, or named as not part of @gsd/core
    ("commands/gsd/ns-review.md", 6, 6),
    ("gsd-core/references/planning-config.md", 293, 293),
    ("gsd-core/references/planning-config.md", 306, 306),
    ("gsd-core/workflows/autonomous.md", 455, 455),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        19,
        19,
    ),
    ("gsd-core/workflows/ingest-docs.md", 23, 23),
    ("gsd-core/workflows/new-milestone.md", 264, 268),
    ("gsd-core/workflows/plan-phase.md", 472, 472),
    (
        "gsd-core/workflows/quick/steps/quick-verification.md",
        55,
        55,
    ),
    ("gsd-core/workflows/smart-entry.md", 55, 55),
];
