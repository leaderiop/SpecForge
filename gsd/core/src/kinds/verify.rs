//! The kinds of verification (`spec/verify.md`): a phase's verification, its gaps, a human's overrides and the items
//! only a human can check. The statuses are computed (`passes::derive::verify`), never written.

use super::dsl::*;

const VERIFICATION_STATUS: &[&str] = &["pending", "passed", "gaps_found", "human_needed"];
const SEVERITY: &[&str] = &["blocker", "warning", "info"];
const GAP_STATUS: &[&str] = &["open", "closed"];

const DEFERRED: &[Field] = &[
    rf("truth", "truth").r(),
    rf("addressed_in", "phase").r(),
    str("evidence").r(),
];
// The ids are `str`: a re-verification names truths, artifacts and key links, and a reference has one target kind.
const RE_VERIFICATION: &[Field] = &[
    en("previous_status", VERIFICATION_STATUS),
    str("previous_score"),
    strs("gaps_closed"),
    strs("gaps_remaining"),
    strs("regressions"),
];
const ANTI_PATTERNS: &[Field] = &[
    str("file"),
    int("line"),
    str("text"),
    en("severity", SEVERITY),
];
const GAP_ARTIFACTS: &[Field] = &[str("path"), str("issue")];

/// `NN-VERIFICATION.md`, named `01-VERIFICATION` and scoped by its phase.
pub const VERIFICATION: Kind = Kind::new(
    "verification",
    "What a phase's verifier found (GSD NN-VERIFICATION.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase:`"),
        blocks("deferred", DEFERRED).doc("`deferred:`"),
        en("status", VERIFICATION_STATUS)
            .c()
            .doc("`status:`; `passed` needs a fresh gsd/E501 verdict on every truth"),
        str("score").c().doc("`score:` as `N/M`"),
        ts("verified")
            .c()
            .doc("`verified:`; the latest verdict's time"),
        files("covered_files")
            .c()
            .doc("the files the phase's verdicts name"),
        str("covered_digest")
            .c()
            .doc("one digest over the covered files' current digests (every E501 judgement of the phase, stale ones too)"),
        block("re_verification", RE_VERIFICATION)
            .c()
            .doc("`re_verification:` against the git baseline"),
        blocks("anti_patterns", ANTI_PATTERNS)
            .c()
            .doc("the lexical anti-pattern scan"),
        bool("scan_partial")
            .c()
            .doc("a file of the phase was not served (a budget, a cap): the scan is partial and the phase is not `passed`"),
    ],
)
.scoped_by("phase");

/// A failed or partial truth as an entity a gap-closure plan can name, `VG1`.
pub const VERIFY_GAP: Kind = Kind::new(
    "verify_gap",
    "A failed or partial truth a gap-closure plan can name (GSD verification gaps)",
    &[
        rf("verification", "verification").r().i().doc("parent"),
        rf("truth", "truth").r().doc("the truth that failed"),
        str("reason").r().doc("why it failed"),
        blocks("artifacts", GAP_ARTIFACTS).doc("the files at fault"),
        strs("missing").doc("what is missing"),
        en("status", GAP_STATUS)
            .c()
            .doc("closed when a `gap_closure` plan naming it is complete"),
        rf("closed_by", "plan").c().doc("that plan"),
    ],
)
.scoped_by("verification");

/// A human's decision to accept a truth that the verifier did not verify, `OV1`. Only a person writes it (ADR 0289);
/// `by` and `at` are not required so that gsd/E503 is the one diagnostic of their absence. Every field but the parent
/// is `.u()`, a human's, and so is the kind itself: the host refuses an agent's write to an existing override at the kind
/// (ADR 0298) and the field's writer on a create, so an agent cannot re-point or forge one (the parent is the entity's
/// scope, which no edit changes).
pub const OVERRIDE: Kind = Kind::new(
    "override",
    "A human accepts a truth the verifier could not verify (GSD verification override)",
    &[
        rf("verification", "verification").r().i().doc("parent"),
        rf("truth", "truth").r().u().doc("the truth accepted"),
        str("accepts").u().doc("the truth's text as the person accepted it; the override counts only while the truth still says this. Without it the override is bound by the truth's id alone"),
        str("reason").r().u().doc("why"),
        str("by").u().doc("who accepted it (gsd/E503)"),
        date("at").u().doc("when (gsd/E503)"),
    ],
)
.human()
.scoped_by("verification");

/// A check only a human can make, `HI1`.
pub const HUMAN_ITEM: Kind = Kind::new(
    "human_item",
    "A check only a human can make (GSD human_verification)",
    &[
        rf("verification", "verification").r().i().doc("parent"),
        str("test").r().doc("what to check"),
        str("expected").r().doc("what should happen"),
        str("why_human").r().doc("why a program cannot check it"),
        rf("uat", "uat_test").doc("the UAT test that asks it"),
    ],
)
.scoped_by("verification");
