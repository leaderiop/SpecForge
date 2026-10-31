//! Broken windows and refactor proposals (`spec/verify.md`). A waiver, a fix and a proposal's scores are harness
//! values, written by their commands (r41) and immutable once recorded; statuses are computed.

use gsd_kit::dsl::*;

const WINDOW_KIND: &[&str] = &[
    "stub",
    "todo",
    "fixme",
    "skipped_test",
    "lint_warning",
    "unmet_truth",
    "unrun_verify",
    "deviation",
];
const WINDOW_STATUS: &[&str] = &["open", "waived", "fixed"];
const REASONS: &[&str] = &["threshold", "jump"];
const PROPOSAL_STATUS: &[&str] = &["proposed", "accepted", "declined"];
const OUTCOME: &[&str] = &["accepted", "declined"];

const WAIVER: &[Field] = &[str("reason").r(), str("by").r(), ts("at").r()];
const CANDIDATES: &[Field] = &[
    str("file"),
    str("function"),
    int("line"),
    int("score"),
    int("baseline"),
    int("delta"),
];
const DISPOSITION: &[Field] = &[
    en("outcome", OUTCOME).r(),
    str("reason"),
    str("by").r(),
    ts("at").r(),
];

/// A line of `.planning/WINDOWS.md`, named `BW1` in the project scope.
// Paths are `str`: the file a window names may since have been deleted.
pub const BROKEN_WINDOW: Kind = Kind::new(
    "broken_window",
    "A defect left in the code, open until it is fixed or waived (GSD WINDOWS.md)",
    &[
        en("kind", WINDOW_KIND).r().doc("`kind`"),
        rf("phase", "phase").r().doc("the phase that found it"),
        str("file").doc("`file`"),
        int("line").doc("`line`"),
        str("description").r().doc("`description`"),
        block("waiver", WAIVER)
            .h()
            .i()
            .doc("`waive` (a reason is required); written by the command that waives it"),
        ts("fixed_at")
            .h()
            .i()
            .doc("`fix`; written by the command that fixes it"),
        rf("refactor", "refactor_proposal")
            .doc("a strict-mode refactor proposal recorded as a `deviation`"),
        en("status", WINDOW_STATUS).c().doc(
            "waived when `waiver` is set; fixed when `fixed_at` is set or its proposal is disposed",
        ),
        rf("milestone", "milestone").c().doc("from the phase"),
        ts("recorded_at")
            .h()
            .i()
            .doc("when the window was recorded"),
    ],
);

/// `NN-REFACTOR.md`, named `RP1` and scoped by its phase. The scores are harness values written by the refactor
/// trigger (r41): a harness field is never `R`.
pub const REFACTOR_PROPOSAL: Kind = Kind::new(
    "refactor_proposal",
    "A function whose complexity crossed the refactor threshold (GSD NN-REFACTOR.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase`"),
        str("target_file").r().i().doc("`target_file`"),
        str("target_function").r().i().doc("`target_function`"),
        int("score").h().i().doc("`score` (decision points)"),
        int("baseline")
            .h()
            .i()
            .doc("`baseline`: the function's anchor score"),
        int("delta").h().i().doc("`delta`"),
        ens("reasons", REASONS).h().i().doc("`reason`"),
        blocks("candidates", CANDIDATES).h().i().doc("`candidates`"),
        block("disposition", DISPOSITION)
            .h()
            .doc("`refactor accept|decline`"),
        en("status", PROPOSAL_STATUS).c().doc("from `disposition`"),
    ],
)
.scoped_by("phase");
