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
];
