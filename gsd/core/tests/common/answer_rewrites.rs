//! Lines (`(path, first, last)`, inclusive, 0-based body lines) that moved into a `specforge gsd` answer although
//! upstream's names no tool the port rule sees. `port_workflows` reads them as more `REWRITTEN_SPANS` rows.

pub const SPANS: &[(&str, usize, usize)] = &[
    ("gsd-core/references/worktree-path-safety.md", 168, 168),
    ("gsd-core/references/worktree-path-safety.md", 137, 137),
    ("gsd-core/references/worktree-path-safety.md", 136, 136),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        275,
        275,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        273,
        273,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        271,
        271,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        270,
        270,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        268,
        268,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        267,
        267,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        266,
        266,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        265,
        265,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        264,
        264,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        256,
        256,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        254,
        254,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        252,
        252,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        249,
        249,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        247,
        247,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        246,
        246,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        244,
        244,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        242,
        242,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        239,
        239,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        238,
        238,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        237,
        237,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        236,
        236,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        235,
        235,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        234,
        234,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        224,
        224,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        222,
        222,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        220,
        220,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        217,
        217,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        215,
        215,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        214,
        214,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        212,
        212,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        210,
        210,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        207,
        207,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        206,
        206,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        205,
        205,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        204,
        204,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        203,
        203,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        202,
        202,
    ),
    ("gsd-core/workflows/complete-milestone.md", 216, 216),
    ("gsd-core/references/planning-config.md", 213, 213),
    ("gsd-core/workflows/ship.md", 87, 87),
    (
        "gsd-core/workflows/execute-phase/steps/protected-branch.md",
        20,
        20,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/protected-branch.md",
        12,
        12,
    ),
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
    // The branching steps read the computed `branch` and the strategy from the settings
    ("gsd-core/workflows/execute-phase.md", 296, 296),
    ("gsd-core/workflows/execute-phase.md", 300, 300),
    ("gsd-core/workflows/execute-phase.md", 308, 308),
    ("gsd-core/workflows/ship.md", 47, 47),
    ("gsd-core/references/planning-config.md", 196, 196),
    ("gsd-core/references/planning-config.md", 198, 198),
    ("gsd-core/references/planning-config.md", 200, 200),
    ("gsd-core/references/planning-config.md", 203, 203),
    ("gsd-core/references/planning-config.md", 205, 205),
    ("gsd-core/references/planning-config.md", 207, 207),
    ("gsd-core/references/planning-config.md", 215, 216),
    ("gsd-core/references/planning-config.md", 222, 223),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        159,
        159,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        168,
        169,
    ),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        175,
        176,
    ),
    ("gsd-core/workflows/complete-milestone.md", 524, 524),
    // Step 0c asks `automated-guard`, and its stated boundary is the limits left
    ("gsd-core/references/worktree-path-safety.md", 192, 399),
    ("gsd-core/references/worktree-path-safety.md", 409, 412),
    ("gsd-core/references/worktree-path-safety.md", 416, 492),
    ("gsd-core/references/worktree-path-safety.md", 494, 497),
];
