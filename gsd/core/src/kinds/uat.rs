//! The kinds of user acceptance testing (`spec/verify.md`): a session, its tests and the gaps a test found. Results
//! are harness values (a person's answer); statuses and totals are computed. `uat_gap.debug` is `@gsd/ops`'s.

use super::dsl::*;

const UAT_STATUS: &[&str] = &["testing", "partial", "complete", "diagnosed"];
const RESULT: &[&str] = &["pending", "pass", "issue", "skipped", "blocked"];
const SOURCE: &[&str] = &["human", "automated"];
const SEVERITY: &[&str] = &["blocker", "major", "minor", "cosmetic"];
const BLOCKED_BY: &[&str] = &[
    "server",
    "physical_device",
    "release_build",
    "third_party",
    "prior_phase",
];
const GAP_STATUS: &[&str] = &["open", "resolved"];

const TOTALS: &[Field] = &[
    int("total"),
    int("passed"),
    int("issues"),
    int("pending"),
    int("skipped"),
    int("blocked"),
];
const GAP_ARTIFACTS: &[Field] = &[str("path"), str("issue")];

/// `NN-UAT.md`, named `01-UAT` and scoped by its phase.
pub const UAT_SESSION: Kind = Kind::new(
    "uat_session",
    "A user acceptance session of a phase (GSD NN-UAT.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase:`"),
        rfs("source", "summary")
            .r()
            .i()
            .doc("`source:`; the summaries tested"),
        ts("started").i().doc("`started:`"),
        en("status", UAT_STATUS).c().doc("`status:`"),
        block("totals", TOTALS).c().doc("`## Summary`"),
    ],
)
.scoped_by("phase");

/// `### N. [Test Name]`, named `U1` and scoped by its session.
pub const UAT_TEST: Kind = Kind::new(
    "uat_test",
    "One thing a person checks (GSD UAT test)",
    &[
        rf("session", "uat_session").r().i().doc("parent"),
        str("expected").r().doc("expected behaviour"),
        int("criterion").doc("the phase success criterion it proves"),
        rf("from_task", "task").doc("a harvested `<human-check>`"),
        en("result", RESULT)
            .h()
            .doc("written from the person's answer"),
        en("source", SOURCE).h().doc(
            "an automated result is marked by the caller (a collector, or `--source automated`)",
        ),
        str("reported").doc("the person's words, verbatim, when `issue`"),
        en("severity", SEVERITY).doc("major when absent"),
        str("reason").doc("why skipped; `Deferred follow-up` counts as passing"),
        en("blocked_by", BLOCKED_BY).doc("when blocked"),
    ],
)
.scoped_by("session");

/// `## Gaps`, named `G1` and scoped by its test.
pub const UAT_GAP: Kind = Kind::new(
    "uat_gap",
    "What a UAT test found wrong (GSD UAT gap)",
    &[
        rf("test", "uat_test").r().i().doc("parent"),
        str("truth").r().doc("`truth:`"),
        str("reason").doc("`reason:`"),
        en("severity", SEVERITY).doc("`severity:`"),
        str("root_cause").doc("`root_cause:`"),
        blocks("artifacts", GAP_ARTIFACTS).doc("`artifacts:`"),
        strs("missing").doc("`missing:`"),
        en("status", GAP_STATUS)
            .c()
            .doc("resolved once a `gap_closure` plan naming it completes"),
        rf("resolved_by", "plan").c().doc("that plan"),
        ts("resolved_at")
            .c()
            .doc("that plan's summary's completion"),
    ],
)
.scoped_by("test");
