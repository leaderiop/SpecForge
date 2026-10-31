//! Plan 03 (F9): the body lines (0-based) of the workflow assets that read JSON with jq, or check a path with bash,
//! where upstream's run node. `port_workflows` reads them as more `REWRITTEN` and `REWRITTEN_SPANS` rows.

pub const LINES: &[(&str, usize)] = &[
    // Plan 03 (F9) T2: the `_gsd_field` helper reads its field with jq; upstream's runs node.
    ("gsd-core/workflows/autonomous.md", 165),
    ("gsd-core/workflows/autonomous.md", 612),
    ("gsd-core/workflows/autonomous.md", 738),
    ("gsd-core/workflows/complete-milestone.md", 341),
    ("gsd-core/workflows/complete-milestone.md", 363),
    ("gsd-core/workflows/complete-milestone.md", 406),
    ("gsd-core/workflows/complete-milestone.md", 432),
    ("gsd-core/workflows/new-milestone.md", 277),
    ("gsd-core/workflows/new-milestone.md", 293),
    ("gsd-core/workflows/new-milestone.md", 535),
    ("gsd-core/workflows/new-milestone.md", 624),
    ("gsd-core/workflows/plan-phase.md", 619),
    // Plan 03 (F9) T3: a field of a JSON answer is read with jq; upstream's line runs node.
    ("gsd-core/references/dispatch-isolation-gate.md", 77),
    ("gsd-core/references/specless-probe-fallback.md", 97),
    ("gsd-core/references/specless-probe-fallback.md", 110),
    ("gsd-core/references/specless-probe-fallback.md", 115),
    ("gsd-core/workflows/execute-phase.md", 225),
    // the TDD gate's line above prints one sha, not JSON: the commit is shown with git
    ("gsd-core/workflows/execute-phase.md", 253),
    (
        "gsd-core/workflows/execute-phase/steps/completion-reconciliation.md",
        35,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md",
        117,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md",
        329,
    ),
    ("gsd-core/workflows/execute-plan.md", 562),
    ("gsd-core/workflows/pr-branch.md", 87),
    ("gsd-core/workflows/pr-branch.md", 88),
    ("gsd-core/workflows/pr-branch.md", 176),
    ("gsd-core/workflows/quick-batch/steps/batch-init.md", 44),
    ("gsd-core/workflows/quick-batch/steps/batch-init.md", 45),
    ("gsd-core/workflows/quick-batch/steps/planner-wave.md", 38),
    ("gsd-core/workflows/quick-batch/steps/research-phase.md", 13),
    ("gsd-core/workflows/quick-batch/steps/resume-mode.md", 30),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        25,
    ),
    ("gsd-core/workflows/quick.md", 603),
    ("gsd-core/workflows/quick.md", 606),
    ("gsd-core/workflows/spec-phase.md", 278),
    ("gsd-core/workflows/spec-phase.md", 301),
    ("gsd-core/workflows/spec-phase.md", 320),
    ("gsd-core/workflows/spec-phase.md", 321),
];

pub const SPANS: &[(&str, usize, usize)] = &[
    // Plan 03 (F9) T3: multi-line node scripts, here jq (a JSON read) or bash (a path or git check).
    ("gsd-core/workflows/pr-branch.md", 95, 126),
    ("gsd-core/workflows/pr-branch.md", 181, 183),
    ("gsd-core/workflows/pr-branch.md", 191, 199),
    ("gsd-core/workflows/review.md", 678, 692),
    ("gsd-core/workflows/ship.md", 436, 441),
];
