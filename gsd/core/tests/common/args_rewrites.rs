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
];
