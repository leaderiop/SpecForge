//! The body lines (0-based) rewritten because what they described is a relic here (ADR 0343): a guard for something
//! SpecForge never prints, a branch no input reaches, a read of a file the graph replaced. `port_workflows` reads them
//! as more `REWRITTEN` and `REWRITTEN_SPANS` rows; a span opens with a line that says what replaced it.

pub const LINES: &[(&str, usize)] = &[
    // SpecForge never prints an `@file:` payload: the prose that taught its guard is gone with the guard.
    ("gsd-core/workflows/complete-milestone.md", 71),
    (
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
        11,
    ),
];

/// `(path, first, last)`, inclusive.
pub const SPANS: &[(&str, usize, usize)] = &[
    // Branches no input reaches (ADR 0343 D4): todos are @gsd/ops entities, a requirement is complete when a summary
    // lists it, and quick commits every spec change with the work.
    ("gsd-core/workflows/execute-phase.md", 1352, 1352),
    ("gsd-core/workflows/execute-phase.md", 1355, 1373),
    ("gsd-core/workflows/execute-phase.md", 1375, 1376),
    ("gsd-core/workflows/execute-plan.md", 522, 522),
    ("gsd-core/workflows/execute-plan.md", 524, 524),
    ("gsd-core/workflows/execute-plan.md", 527, 527),
    ("gsd-core/workflows/quick.md", 719, 720),
    ("gsd-core/workflows/quick.md", 723, 723),
    ("gsd-core/workflows/quick.md", 725, 728),
    // pr-branch has one mode (strict), no sub-repositories and no third bucket: every spec path is filtered.
    ("gsd-core/workflows/pr-branch.md", 62, 62),
    ("gsd-core/workflows/pr-branch.md", 77, 78),
    ("gsd-core/workflows/pr-branch.md", 80, 80),
    ("gsd-core/workflows/pr-branch.md", 82, 85),
    ("gsd-core/workflows/pr-branch.md", 87, 89),
    ("gsd-core/workflows/pr-branch.md", 91, 93),
    ("gsd-core/workflows/pr-branch.md", 95, 97),
    ("gsd-core/workflows/pr-branch.md", 101, 126),
    ("gsd-core/workflows/pr-branch.md", 128, 129),
    ("gsd-core/workflows/pr-branch.md", 131, 131),
    ("gsd-core/workflows/pr-branch.md", 133, 133),
    ("gsd-core/workflows/pr-branch.md", 135, 138),
    ("gsd-core/workflows/pr-branch.md", 140, 144),
    ("gsd-core/workflows/pr-branch.md", 146, 146),
    ("gsd-core/workflows/pr-branch.md", 148, 151),
    ("gsd-core/workflows/pr-branch.md", 153, 157),
    ("gsd-core/workflows/pr-branch.md", 162, 163),
    ("gsd-core/workflows/pr-branch.md", 165, 168),
    ("gsd-core/workflows/pr-branch.md", 170, 174),
    ("gsd-core/workflows/pr-branch.md", 176, 178),
    ("gsd-core/workflows/pr-branch.md", 180, 181),
    ("gsd-core/workflows/pr-branch.md", 185, 199),
    ("gsd-core/workflows/pr-branch.md", 201, 215),
    ("gsd-core/workflows/pr-branch.md", 217, 226),
    ("gsd-core/workflows/pr-branch.md", 228, 229),
    ("gsd-core/workflows/pr-branch.md", 262, 262),
    ("gsd-core/workflows/pr-branch.md", 275, 275),
    ("gsd-core/workflows/pr-branch.md", 278, 279),
    ("gsd-core/workflows/pr-branch.md", 309, 309),
    ("gsd-core/workflows/pr-branch.md", 318, 318),
    ("gsd-core/workflows/pr-branch.md", 322, 327),
    ("gsd-core/workflows/pr-branch.md", 329, 330),
    ("gsd-core/workflows/pr-branch.md", 332, 339),
    ("gsd-core/workflows/pr-branch.md", 341, 341),
    ("gsd-core/workflows/pr-branch.md", 345, 345),
    ("gsd-core/workflows/pr-branch.md", 348, 349),
    ("gsd-core/workflows/pr-branch.md", 401, 424),
    ("gsd-core/workflows/pr-branch.md", 426, 429),
    ("gsd-core/workflows/pr-branch.md", 432, 439),
    ("gsd-core/workflows/pr-branch.md", 441, 452),
    ("gsd-core/workflows/pr-branch.md", 498, 498),
    ("gsd-core/workflows/pr-branch.md", 500, 501),
    ("gsd-core/workflows/pr-branch.md", 519, 520),
    ("gsd-core/workflows/pr-branch.md", 550, 551),
    ("gsd-core/workflows/pr-branch.md", 553, 553),
    ("gsd-core/workflows/pr-branch.md", 555, 555),
    ("gsd-core/workflows/pr-branch.md", 564, 564),
];
