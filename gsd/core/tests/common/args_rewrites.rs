//! The body lines (0-based) rewritten because a workflow now reads a prompt's typed argument through an `{{arg:}}`
//! placeholder instead of re-parsing the argument text (ADR 0340). `port_workflows` reads them as more
//! `REWRITTEN_SPANS` rows; the port rule's `drops_an_argument_capture` covers an emptied capture line and a dropped
//! `{GSD_WS}` token without a row.

/// `(path, first, last)`, inclusive.
pub const SPANS: &[(&str, usize, usize)] = &[
    // The target is the typed `--target`, not a parse of the argument text.
    ("gsd-core/workflows/pr-branch.md", 23, 23),
    // new-milestone binds the workstream and the name from `ws` and `name`; `--reset-phase-numbers` has no reader.
    ("gsd-core/workflows/new-milestone.md", 26, 27),
    ("gsd-core/workflows/new-milestone.md", 29, 29),
    ("gsd-core/workflows/new-milestone.md", 37, 37),
    ("gsd-core/workflows/new-milestone.md", 306, 306),
    (
        "gsd-core/workflows/new-milestone/steps/project-md-milestone-write.md",
        0,
        0,
    ),
    // quick-batch reads its typed values and checks `--jobs`; the host refuses `--discuss` and `--full`.
    ("gsd-core/workflows/quick-batch.md", 39, 39),
    ("gsd-core/workflows/quick-batch.md", 42, 51),
    ("gsd-core/workflows/quick-batch.md", 55, 55),
    ("gsd-core/workflows/quick-batch.md", 57, 57),
    ("commands/gsd/quick-batch.md", 50, 59),
    ("commands/gsd/quick-batch.md", 61, 68),
    ("commands/gsd/quick-batch.md", 70, 73),
    ("commands/gsd/quick-batch.md", 75, 78),
    ("commands/gsd/quick-batch.md", 98, 98),
    ("gsd-core/workflows/quick-batch/steps/batch-init.md", 5, 6),
    // autonomous, progress's next.md and convergence read their declared flags typed; the undeclared reviewer lanes,
    // --max-cycles and --no-transition are dropped; convergence reads --override-gate and each lane as a flag.
    ("gsd-core/workflows/autonomous.md", 20, 20),
    ("gsd-core/workflows/autonomous.md", 30, 33),
    ("gsd-core/workflows/autonomous.md", 35, 38),
    ("gsd-core/workflows/autonomous.md", 40, 44),
    ("gsd-core/workflows/autonomous.md", 46, 49),
    ("gsd-core/workflows/autonomous.md", 51, 54),
    ("gsd-core/workflows/autonomous.md", 56, 63),
    ("gsd-core/workflows/autonomous.md", 70, 70),
    ("gsd-core/workflows/autonomous.md", 96, 97),
    ("gsd-core/workflows/autonomous.md", 99, 99),
    ("gsd-core/workflows/autonomous.md", 102, 105),
    ("gsd-core/workflows/autonomous.md", 107, 111),
    ("gsd-core/workflows/autonomous.md", 402, 402),
    ("gsd-core/workflows/autonomous.md", 411, 411),
    ("gsd-core/workflows/autonomous.md", 417, 417),
    ("gsd-core/workflows/autonomous.md", 505, 505),
    ("gsd-core/workflows/autonomous.md", 806, 806),
    ("gsd-core/workflows/next.md", 277, 280),
    ("gsd-core/workflows/next.md", 283, 284),
    ("gsd-core/workflows/next.md", 286, 289),
    ("gsd-core/workflows/next.md", 291, 295),
    ("gsd-core/workflows/plan-review-convergence.md", 20, 20),
    ("gsd-core/workflows/plan-review-convergence.md", 29, 29),
    ("gsd-core/workflows/plan-review-convergence.md", 35, 36),
    ("gsd-core/workflows/plan-review-convergence.md", 39, 39),
    ("gsd-core/workflows/plan-review-convergence.md", 55, 56),
    ("gsd-core/workflows/plan-review-convergence.md", 94, 97),
    ("gsd-core/workflows/plan-review-convergence.md", 608, 608),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        46,
        46,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        49,
        49,
    ),
    // plan-phase and execute-phase read their declared flags typed; their unread *_PARAM values, the undeclared
    // --auto, --mvp and --force branches and the closed-phase --force advice go.
    ("gsd-core/workflows/plan-phase.md", 77, 82),
    ("gsd-core/workflows/plan-phase.md", 87, 88),
    ("gsd-core/workflows/plan-phase.md", 119, 119),
    ("gsd-core/workflows/plan-phase.md", 137, 139),
    ("gsd-core/workflows/plan-phase.md", 141, 141),
    ("gsd-core/workflows/plan-phase.md", 143, 143),
    ("gsd-core/workflows/plan-phase.md", 160, 161),
    ("gsd-core/workflows/plan-phase.md", 166, 166),
    ("gsd-core/workflows/plan-phase.md", 168, 168),
    ("gsd-core/workflows/plan-phase.md", 215, 215),
    ("gsd-core/workflows/execute-phase.md", 93, 93),
    ("gsd-core/workflows/execute-phase.md", 191, 192),
    ("gsd-core/workflows/execute-phase.md", 194, 194),
    ("gsd-core/workflows/execute-phase.md", 203, 204),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        4,
        4,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        12,
        14,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        17,
        17,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
        32,
        33,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        15,
        15,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        17,
        17,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        30,
        30,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
        32,
        32,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
        129,
        129,
    ),
    // milestone-summary takes its milestone typed, else the one the run is about.
    ("gsd-core/workflows/milestone-summary.md", 16, 16),
    // The inlined workflows' dead captures: unread *_PARAM and PHASE_ARG values, the --ws parse; the prose names the
    // `$PHASE_ID` the prompt's tail binds.
    ("gsd-core/workflows/add-tests.md", 14, 14),
    ("gsd-core/workflows/add-tests.md", 17, 17),
    ("gsd-core/workflows/add-tests.md", 51, 51),
    ("gsd-core/workflows/add-tests.md", 63, 63),
    ("gsd-core/workflows/extract-learnings.md", 25, 25),
    ("gsd-core/workflows/extract-learnings.md", 33, 33),
    ("gsd-core/workflows/progress.md", 21, 21),
    ("gsd-core/workflows/quick.md", 123, 126),
    ("gsd-core/workflows/review.md", 153, 153),
    ("gsd-core/workflows/ship.md", 32, 32),
    ("gsd-core/workflows/validate-phase.md", 24, 24),
    ("gsd-core/workflows/verify-work.md", 47, 48),
    ("gsd-core/workflows/new-project.md", 37, 37),
    // The read-loaded files and complete-milestone: the --ws test, the unread --auto parse, the dead chain-flag
    // branches and the comments about the parent's argument text go with the captures.
    ("gsd-core/workflows/complete-milestone.md", 256, 256),
    ("gsd-core/workflows/discuss-phase-assumptions.md", 70, 70),
    ("gsd-core/workflows/discuss-phase-assumptions.md", 678, 678),
    ("gsd-core/workflows/discuss-phase-assumptions.md", 680, 680),
    ("gsd-core/workflows/discuss-phase/modes/chain.md", 34, 34),
    ("gsd-core/workflows/discuss-phase/modes/chain.md", 36, 36),
    (
        "gsd-core/workflows/execute-phase/steps/codebase-drift-gate.md",
        73,
        76,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        29,
        29,
    ),
    (
        "gsd-core/workflows/verify-work/steps/mvp-uat-framing.md",
        15,
        15,
    ),
    // No text forwards a workstream: the #4456 persistence comments, the routing contract and the unread
    // GSD_WS_ARG lines say the ws argument is read where it is declared and forwarded nowhere.
    ("gsd-core/references/phase-argument-parsing.md", 6, 6),
    ("gsd-core/references/workstream-flag.md", 79, 83),
    ("gsd-core/references/workstream-flag.md", 85, 91),
    ("gsd-core/workflows/new-milestone.md", 40, 42),
    ("gsd-core/workflows/new-milestone.md", 54, 57),
    ("gsd-core/workflows/new-milestone.md", 62, 62),
    ("gsd-core/workflows/new-milestone.md", 175, 175),
    ("gsd-core/workflows/new-milestone.md", 178, 178),
    ("gsd-core/workflows/new-milestone.md", 315, 317),
    ("gsd-core/workflows/new-milestone.md", 629, 630),
];
