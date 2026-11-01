//! G3 (plan 02): the body lines (0-based, inclusive runs) of the quick workflows rewritten because a quick plan's
//! context, decisions and research are entities that name it in `plan`: the discussion and research steps create
//! them, the reading lists hand a sub-agent a `specforge query` of the plan, context or research (upstream's
//! parenthesis kept), and the existence checks query the entity. A plan counts as planned when it holds a task, as
//! the discussion or research step may have made it first. Prose that only names a file, the completion banners and
//! the planner's "Write plan to" line stay upstream's. `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    ("gsd-core/workflows/quick.md", 284, 284),
    ("gsd-core/workflows/quick.md", 325, 326),
    ("gsd-core/workflows/quick.md", 338, 338),
    ("gsd-core/workflows/quick.md", 362, 362),
    ("gsd-core/workflows/quick.md", 364, 364),
    ("gsd-core/workflows/quick.md", 366, 366),
    ("gsd-core/workflows/quick.md", 480, 482),
    ("gsd-core/workflows/quick.md", 564, 564),
    ("gsd-core/workflows/quick.md", 570, 570),
    ("gsd-core/workflows/quick/steps/discussion-phase.md", 40, 40),
    ("gsd-core/workflows/quick/steps/discussion-phase.md", 69, 69),
    ("gsd-core/workflows/quick/steps/discussion-phase.md", 71, 71),
    (
        "gsd-core/workflows/quick/steps/discussion-phase.md",
        121,
        121,
    ),
    ("gsd-core/workflows/quick/steps/research-phase.md", 11, 11),
    ("gsd-core/workflows/quick/steps/research-phase.md", 28, 28),
    ("gsd-core/workflows/quick/steps/research-phase.md", 34, 34),
    ("gsd-core/workflows/quick/steps/research-phase.md", 52, 52),
    ("gsd-core/workflows/quick/steps/research-phase.md", 66, 67),
    (
        "gsd-core/workflows/quick/steps/plan-checker-loop.md",
        32,
        32,
    ),
    (
        "gsd-core/workflows/quick/steps/plan-checker-loop.md",
        104,
        104,
    ),
    (
        "gsd-core/workflows/quick/steps/quick-verification.md",
        18,
        18,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/plan-checker-loop.md",
        39,
        39,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/plan-checker-loop.md",
        87,
        87,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
        15,
        16,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
        90,
        90,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
        100,
        100,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/planner-wave.md",
        128,
        128,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/verification-wave.md",
        32,
        32,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        47,
        47,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        101,
        101,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        163,
        163,
    ),
    ("gsd-core/workflows/quick-batch/steps/merge-wave.md", 10, 10),
    ("gsd-core/workflows/quick-batch/steps/merge-wave.md", 35, 35),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        17,
        17,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        24,
        24,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        55,
        55,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        76,
        76,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        89,
        89,
    ),
];
