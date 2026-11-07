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
    // an optional integration routes by what the project serves: the planner's graph context
    ("gsd-core/references/planner-load-graph-context.md", 4, 6),
    ("gsd-core/references/planner-load-graph-context.md", 8, 11),
    ("gsd-core/references/planner-load-graph-context.md", 13, 13),
    ("gsd-core/references/planner-load-graph-context.md", 15, 15),
    ("gsd-core/references/planner-load-graph-context.md", 17, 17),
    ("gsd-core/references/planner-load-graph-context.md", 19, 19),
    ("gsd-core/references/planner-load-graph-context.md", 21, 28),
    ("gsd-core/references/planner-load-graph-context.md", 30, 30),
    ("gsd-core/references/planner-load-graph-context.md", 32, 32),
    ("gsd-core/references/planner-load-graph-context.md", 35, 35),
    ("gsd-core/workflows/plan-phase.md", 765, 765),
    // an optional program runs through specforge run, with no probe: ctx7
    (
        "gsd-core/references/research-documentation-lookup.md",
        13,
        17,
    ),
    (
        "gsd-core/references/research-documentation-lookup.md",
        21,
        25,
    ),
    (
        "gsd-core/references/research-documentation-lookup.md",
        28,
        30,
    ),
    // a host chore is the user's: PowerShell recovery in a text fence, the port and env checks in prose
    ("gsd-core/references/checkpoints.md", 499, 499),
    ("gsd-core/references/verification-patterns.md", 311, 312),
    ("gsd-core/references/verification-patterns.md", 315, 321),
    (
        "gsd-core/workflows/plan-phase/steps/windows-troubleshooting.md",
        6,
        7,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/windows-troubleshooting.md",
        11,
        12,
    ),
    // a file in the roots is read with specforge files read
    (
        "gsd-core/workflows/plan-phase/steps/prd-express-path.md",
        8,
        8,
    ),
    ("gsd-core/workflows/review.md", 333, 334),
    ("gsd-core/workflows/review.md", 536, 536),
    ("gsd-core/workflows/ship.md", 424, 424),
    // a host file outside the roots is read with the host's Read tool
    ("gsd-core/workflows/new-project.md", 305, 305),
    ("gsd-core/workflows/new-project.md", 307, 309),
    // a hand-off file's text match is an answer, and an empty lane file a test
    ("gsd-core/workflows/review.md", 570, 571),
    ("gsd-core/workflows/review.md", 575, 575),
    ("gsd-core/workflows/review.md", 600, 600),
    ("gsd-core/workflows/review.md", 675, 676),
    // a model-held list or literal text is glue: a placeholder, a heredoc read, printf
    ("gsd-core/references/specless-probe-fallback.md", 90, 90),
    ("gsd-core/references/specless-probe-fallback.md", 95, 95),
    (
        "gsd-core/workflows/execute-phase/steps/regression-gate.md",
        32,
        32,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/regression-gate.md",
        36,
        36,
    ),
    ("gsd-core/workflows/execute-plan.md", 468, 472),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        20,
        25,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        29,
        36,
    ),
    ("gsd-core/workflows/quick.md", 202, 204),
    ("gsd-core/workflows/review.md", 400, 400),
    ("gsd-core/workflows/review.md", 499, 499),
    ("gsd-core/workflows/review.md", 570, 570),
    ("gsd-core/workflows/review.md", 582, 582),
    ("gsd-core/workflows/review.md", 643, 643),
    ("gsd-core/workflows/review.md", 648, 648),
    ("gsd-core/workflows/spec-phase.md", 270, 270),
    ("gsd-core/workflows/spec-phase.md", 275, 275),
    // a flag the command does not declare is not offered: the host refuses it
    ("gsd-core/workflows/execute-phase.md", 77, 78),
    ("gsd-core/workflows/execute-phase.md", 437, 437),
    (
        "gsd-core/workflows/execute-phase/detail/elaboration.md",
        64,
        66,
    ),
    ("gsd-core/workflows/next.md", 309, 309),
    ("gsd-core/workflows/next.md", 334, 334),
    ("gsd-core/workflows/next.md", 363, 363),
    ("gsd-core/workflows/next.md", 365, 365),
];
