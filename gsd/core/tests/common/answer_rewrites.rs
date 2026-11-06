//! Lines (`(path, first, last)`, inclusive, 0-based body lines) that moved into a `specforge gsd` answer although
//! upstream's names no tool the port rule sees. `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    // The gates read `gate-command`'s argv: the choice of a command is the answer's, and nothing is re-parsed.
    (
        "gsd-core/workflows/execute-phase/steps/post-merge-gate.md",
        10,
        65,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/post-merge-gate.md",
        68,
        69,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/post-merge-gate.md",
        94,
        148,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/regression-gate-run.md",
        19,
        19,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/regression-gate-run.md",
        23,
        55,
    ),
];
