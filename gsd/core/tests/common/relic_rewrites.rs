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
    // A verification has no next-action or next-command field: each route names its command.
    ("gsd-core/workflows/progress.md", 505),
    ("gsd-core/workflows/progress.md", 509),
    ("gsd-core/workflows/progress.md", 525),
    ("gsd-core/workflows/progress.md", 529),
    ("gsd-core/workflows/progress.md", 545),
    ("gsd-core/workflows/progress.md", 549),
];

/// `(path, first, last)`, inclusive.
pub const SPANS: &[(&str, usize, usize)] = &[
    // Branches no input reaches (ADR 0343 D4): todos are `@gsd/ops` entities, a requirement is complete when a summary
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
    // The pr-branch workflow has one mode (strict), no sub-repositories and no third bucket: every spec path is
    // filtered.
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
    // The state is the graph (ADR 0343 D3): a graph read picks the export's nodes, a phase is its id, and the roadmap
    // is entities: no ROADMAP.md to reorganize, no init-bundle key to read.
    ("gsd-core/workflows/complete-milestone.md", 329, 329),
    ("gsd-core/workflows/complete-milestone.md", 331, 331),
    ("gsd-core/workflows/complete-milestone.md", 341, 344),
    ("gsd-core/workflows/complete-milestone.md", 347, 347),
    ("gsd-core/workflows/complete-milestone.md", 349, 349),
    ("gsd-core/workflows/complete-milestone.md", 353, 353),
    ("gsd-core/workflows/complete-milestone.md", 363, 364),
    ("gsd-core/workflows/complete-milestone.md", 366, 366),
    ("gsd-core/workflows/complete-milestone.md", 371, 371),
    ("gsd-core/workflows/complete-milestone.md", 373, 374),
    ("gsd-core/workflows/complete-milestone.md", 376, 376),
    ("gsd-core/workflows/complete-milestone.md", 378, 379),
    ("gsd-core/workflows/complete-milestone.md", 381, 381),
    ("gsd-core/workflows/complete-milestone.md", 383, 384),
    ("gsd-core/workflows/complete-milestone.md", 386, 387),
    ("gsd-core/workflows/complete-milestone.md", 389, 390),
    ("gsd-core/workflows/complete-milestone.md", 392, 392),
    ("gsd-core/workflows/complete-milestone.md", 394, 394),
    ("gsd-core/workflows/complete-milestone.md", 406, 414),
    ("gsd-core/workflows/new-milestone.md", 270, 270),
    ("gsd-core/workflows/new-milestone.md", 277, 282),
    ("gsd-core/workflows/new-milestone.md", 285, 285),
    ("gsd-core/workflows/new-milestone.md", 293, 295),
    ("gsd-core/workflows/new-milestone.md", 535, 536),
    ("gsd-core/workflows/new-milestone.md", 624, 627),
    ("gsd-core/workflows/plan-phase.md", 113, 113),
    ("gsd-core/workflows/plan-phase.md", 619, 628),
    ("gsd-core/workflows/plan-phase.md", 638, 638),
    ("gsd-core/workflows/plan-phase.md", 643, 643),
    ("gsd-core/workflows/progress.md", 325, 326),
    ("gsd-core/workflows/progress.md", 329, 329),
    ("gsd-core/workflows/verify-work.md", 76, 77),
    ("gsd-core/workflows/verify-work.md", 648, 648),
    ("gsd-core/workflows/verify-work.md", 655, 655),
    ("gsd-core/workflows/verify-work.md", 682, 682),
    ("gsd-core/workflows/verify-work.md", 692, 695),
    // A phase's files are its entities: read by kind, never listed or globbed under a phase directory.
    ("gsd-core/references/autonomous-ui-design-contract.md", 9, 9),
    (
        "gsd-core/references/autonomous-ui-design-contract.md",
        38,
        38,
    ),
    ("gsd-core/references/nyquist-compliance.md", 20, 20),
    ("gsd-core/references/planner-gap-closure.md", 8, 8),
    ("gsd-core/references/planner-gap-closure.md", 12, 12),
    ("gsd-core/references/planner-gap-closure.md", 15, 15),
    ("gsd-core/references/verifier-phase-gates.md", 35, 35),
    ("gsd-core/workflows/autonomous.md", 548, 548),
    ("gsd-core/workflows/discuss-phase-assumptions.md", 123, 123),
    ("gsd-core/workflows/discuss-phase.md", 203, 203),
    ("gsd-core/workflows/plan-phase.md", 300, 300),
    ("gsd-core/workflows/plan-phase.md", 460, 460),
    ("gsd-core/workflows/plan-phase.md", 463, 463),
    ("gsd-core/workflows/plan-phase.md", 543, 543),
    ("gsd-core/workflows/plan-phase.md", 569, 569),
    ("gsd-core/workflows/plan-phase.md", 609, 609),
    ("gsd-core/workflows/plan-phase.md", 664, 664),
    ("gsd-core/workflows/plan-phase.md", 1311, 1311),
    ("gsd-core/workflows/plan-phase.md", 1368, 1368),
    (
        "gsd-core/workflows/plan-phase/steps/research-only-modifiers.md",
        12,
        13,
    ),
    ("gsd-core/workflows/validate-phase.md", 49, 50),
    ("gsd-core/workflows/verify-work.md", 174, 174),
    ("gsd-core/workflows/verify-work.md", 177, 177),
    ("gsd-core/workflows/verify-work.md", 180, 180),
    ("gsd-core/workflows/verify-work.md", 309, 309),
    (
        "gsd-core/workflows/verify-work/steps/automated-ui-verification.md",
        9,
        9,
    ),
    // A path under the phase directory names the entity: the handoff, the context, the validation, the pattern map.
    ("gsd-core/workflows/discuss-phase-assumptions.md", 616, 616),
    ("gsd-core/workflows/discuss-phase.md", 166, 166),
    ("gsd-core/workflows/discuss-phase.md", 482, 482),
    ("gsd-core/workflows/execute-phase.md", 268, 268),
    ("gsd-core/workflows/plan-phase.md", 465, 465),
    ("gsd-core/workflows/plan-phase.md", 469, 469),
    ("gsd-core/workflows/plan-phase.md", 707, 707),
    ("gsd-core/workflows/plan-phase.md", 712, 712),
    ("gsd-core/workflows/validate-phase.md", 141, 141),
    ("gsd-core/workflows/validate-phase.md", 160, 160),
    // The planner's and the checker's required reading names entities read with `specforge`; the project state is
    // computed and REVIEWS.md is no entity, so neither is a placeholder.
    ("gsd-core/workflows/plan-phase.md", 767, 778),
    ("gsd-core/workflows/plan-phase.md", 1048, 1053),
    ("gsd-core/workflows/plan-phase.md", 1177, 1178),
    ("gsd-core/workflows/plan-phase.md", 1460, 1460),
    // Stats, settings and commit counts are the graph's and the settings' answers.
    ("gsd-core/workflows/complete-milestone.md", 188, 192),
    ("gsd-core/workflows/complete-milestone.md", 203, 203),
    ("gsd-core/templates/milestone.md", 53, 54),
    ("gsd-core/workflows/settings-advanced.md", 44, 44),
    ("gsd-core/workflows/settings-advanced.md", 608, 608),
    ("gsd-core/workflows/verify-work.md", 182, 214),
    // The UI gate reads the phase's computed `frontend` field.
    ("gsd-core/workflows/progress.md", 377, 377),
    ("gsd-core/workflows/progress.md", 583, 583),
    ("gsd-core/workflows/new-project.md", 1154, 1154),
    ("gsd-core/workflows/new-project.md", 1158, 1158),
    // A quick-batch item is its quick plan, named by its id: no `quick_dir` is set.
    ("gsd-core/workflows/quick-batch.md", 76, 76),
    ("gsd-core/workflows/quick-batch.md", 83, 84),
    (
        "gsd-core/workflows/quick-batch/steps/research-phase.md",
        23,
        24,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        87,
        87,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        114,
        114,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/verification-wave.md",
        25,
        25,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/verification-wave.md",
        37,
        37,
    ),
    // The disposition ledger, the AI spec's framework line and the user profile are their extensions'.
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        83,
        87,
    ),
    ("gsd-core/workflows/plan-phase.md", 302, 305),
    ("gsd-core/workflows/plan-phase.md", 308, 308),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 6, 6),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 13, 13),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 29, 29),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 47, 47),
    ("gsd-core/workflows/discuss-phase/modes/advisor.md", 50, 50),
    // The default reviewers are a settings pick.
    ("gsd-core/workflows/plan-review-convergence.md", 114, 114),
    ("gsd-core/workflows/plan-review-convergence.md", 124, 125),
    // The model writes the review agent's counts and sections; the shell checks them.
    ("gsd-core/workflows/plan-review-convergence.md", 389, 389),
    ("gsd-core/workflows/plan-review-convergence.md", 392, 400),
    ("gsd-core/workflows/plan-review-convergence.md", 402, 402),
    ("gsd-core/workflows/plan-review-convergence.md", 404, 405),
    ("gsd-core/workflows/plan-review-convergence.md", 407, 407),
    ("gsd-core/workflows/plan-review-convergence.md", 409, 410),
    ("gsd-core/workflows/plan-review-convergence.md", 414, 416),
    // `@gsd/core` runs no worktree isolation: one dispatch template, sequential mode.
    ("gsd-core/workflows/execute-phase.md", 634, 634),
    ("gsd-core/workflows/execute-phase.md", 642, 642),
    ("gsd-core/workflows/execute-phase.md", 690, 690),
    (
        "gsd-core/workflows/execute-phase/steps/sequential-root-pin.md",
        33,
        34,
    ),
    ("gsd-core/workflows/execute-phase.md", 724, 724),
    ("gsd-core/workflows/execute-phase.md", 772, 772),
    ("gsd-core/workflows/execute-phase.md", 822, 822),
    ("gsd-core/workflows/execute-phase.md", 976, 976),
    ("gsd-core/references/dispatch-isolation-gate.md", 127, 128),
    ("gsd-core/references/dispatch-isolation-gate.md", 130, 131),
    // Quick-batch runs no worktree isolation either: its executors dispatch in sequential mode, and nothing is merged.
    ("gsd-core/workflows/quick-batch.md", 4, 4),
    ("gsd-core/workflows/quick-batch.md", 85, 85),
    ("gsd-core/workflows/quick-batch.md", 166, 166),
    ("gsd-core/workflows/quick-batch.md", 169, 171),
    ("gsd-core/workflows/quick-batch.md", 177, 179),
    ("gsd-core/workflows/quick-batch.md", 205, 207),
    ("gsd-core/workflows/quick-batch.md", 209, 209),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        0,
        0,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        2,
        3,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        21,
        22,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        43,
        43,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        47,
        47,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        57,
        57,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        61,
        67,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        79,
        80,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        90,
        91,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        123,
        123,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        160,
        160,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        163,
        165,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/verification-wave.md",
        73,
        75,
    ),
    ("gsd-core/workflows/quick-batch/steps/completion.md", 24, 25),
    // Chunked planning resumes past a plan entity.
    (
        "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
        119,
        119,
    ),
    (
        "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
        128,
        129,
    ),
    // The verification step reads its own inputs: the phase's number and requirement ids from the graph, the response
    // language from the settings. No model is resolved and no section manifest gates a step.
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        14,
        17,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        115,
        117,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        121,
        123,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        132,
        134,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        136,
        136,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        140,
        140,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        144,
        148,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        155,
        155,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
        159,
        159,
    ),
];

/// `(path, first, last)`, inclusive: runs deleted by blanking, whose first line (when not blank) says what replaced
/// them. Every other line of the run is blank; upstream's blank lines stay blank, so they are not listed as rewritten.
pub const BLANKED: &[(&str, usize, usize)] = &[
    // The disposition ledger is `@gsd/ops`'s: `@gsd/core` keeps no REVIEW.md parser.
    (
        "gsd-core/workflows/execute-phase/steps/code-review-disposition.md",
        0,
        1018,
    ),
    // `@gsd/core` runs no worktree isolation: its steps are not reached, its branches are gone.
    (
        "gsd-core/workflows/execute-phase/steps/per-plan-worktree-gate.md",
        0,
        144,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/executor-isolation-dispatch.md",
        0,
        373,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/worktree-base-check.md",
        0,
        24,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/worktree-recovery-policy.md",
        0,
        10,
    ),
    ("gsd-core/workflows/quick-batch/steps/merge-wave.md", 0, 99),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        6,
        19,
    ),
    (
        "gsd-core/workflows/quick-batch/steps/worktree-dispatch.md",
        127,
        158,
    ),
    ("gsd-core/workflows/quick-batch.md", 109, 122),
    ("gsd-core/references/execute-phase-wave-guard.md", 0, 49),
    (
        "gsd-core/references/execute-phase-between-wave-reset.md",
        10,
        47,
    ),
    (
        "gsd-core/workflows/execute-phase/steps/per-plan-executor-routing.md",
        34,
        39,
    ),
    ("gsd-core/workflows/execute-phase.md", 127, 131),
    ("gsd-core/workflows/execute-phase.md", 134, 152),
    ("gsd-core/workflows/execute-phase.md", 607, 614),
    ("gsd-core/workflows/execute-phase.md", 636, 637),
    ("gsd-core/workflows/execute-phase.md", 647, 658),
    ("gsd-core/workflows/execute-phase.md", 661, 672),
    ("gsd-core/workflows/execute-phase.md", 681, 684),
    ("gsd-core/workflows/execute-phase.md", 693, 697),
    ("gsd-core/workflows/execute-phase.md", 764, 767),
    ("gsd-core/workflows/execute-phase.md", 770, 771),
    ("gsd-core/workflows/execute-phase.md", 847, 932),
];

/// Whether body line `n` of `path`, upstream's `theirs`, is a line of a [`BLANKED`] run that upstream did not leave
/// blank.
pub fn blanked(path: &str, n: usize, theirs: &str) -> bool {
    !theirs.is_empty() && super::rule::in_spans(BLANKED, path, n)
}

/// Asserts every line of `path`'s [`BLANKED`] runs after the first is blank.
pub fn assert_blanked(path: &str, asset: &str) {
    let lines = super::rule::body_lines(asset);
    for (_, a, b) in BLANKED.iter().filter(|(p, _, _)| *p == path) {
        for (n, line) in lines.iter().enumerate().take(b + 1).skip(a + 1) {
            assert!(line.is_empty(), "{path}:{n}: a blanked run holds {line}");
        }
    }
}
