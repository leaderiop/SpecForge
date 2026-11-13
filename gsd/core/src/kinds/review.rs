//! The kinds of a cross-AI review (`spec/plan.md`, "Reviews and revision conflicts"): a review round, the concerns
//! it raised, and the revision conflicts a replan reports. A concern's status is computed from the dispositions that
//! name it, so a round is written once.

use gsd_kit::dsl::*;

use super::execute::RESOLVED;

const REVIEWER: &[Field] = &[str("name").r(), str("model").r(), str("source").r()];
const RISK: &[&str] = &["low", "medium", "high"];
const LANE: &[Field] = &[
    str("reviewer").r(),
    str("summary").r(),
    strs("strengths"),
    strs("suggestions"),
    en("risk", RISK),
];
const COVERAGE: &[Field] = &[str("reviewer").r(), int("total"), strs("missing")];
const UNCHECKED: &[Field] = &[str("symbol").r(), str("why").r()];
const OPEN: &[Field] = &[int("high"), int("actionable")];

/// `NN-REVIEWS.md`: one round of `/gsd-review`, named `RV1` and scoped by its phase, in the phase's file.
pub const REVIEW: Kind = Kind::new(
    "review",
    "A cross-AI review round of a phase's plans (GSD NN-REVIEWS.md)",
    &[
        rf("phase", "phase").r().i().doc("`phase:`; parent"),
        rfs("plans", "plan")
            .r()
            .i()
            .doc("`plans_reviewed:`: the phase's plans the round read"),
        blocks("reviewers", REVIEWER).i().doc(
            "`reviewers:`, `models:` and `model_sources:`: one row per reviewer, names unique",
        ),
        blocks("lanes", LANE)
            .i()
            .doc("each reviewer's section: summary, strengths, suggestions and risk"),
        blocks("coverage", COVERAGE).i().doc(
            "`plan_coverage:`: the plans each reviewer read of the total, and the ones it missed",
        ),
        str("consensus").i().doc("`## Consensus Summary`"),
        strs("strengths").i().doc("`### Agreed Strengths`"),
        strs("divergent").i().doc("`### Divergent Views`"),
        ts("reviewed_at")
            .h()
            .doc("`reviewed:`: when `record_review` wrote the round"),
        blocks("unchecked", UNCHECKED)
            .append()
            .doc("the symbols a convergence cycle could not check, and why"),
        block("open", OPEN).c().doc(
            "its open concerns: `high` ones, and `actionable` ones that are neither high nor drift",
        ),
    ],
)
.scoped_by("phase");

const SEVERITY: &[&str] = &["high", "medium", "low"];
const CONCERN_STATUS: &[&str] = &["open", "addressed", "deferred"];

/// One concern of a round, named `RC1` and scoped by its review: append-only, every authored field immutable.
/// `### Agreed Concerns` are the concerns two or more reviewers raised.
pub const REVIEW_CONCERN: Kind = Kind::new(
    "review_concern",
    "A concern a review round raised about a plan (GSD NN-REVIEWS.md concerns)",
    &[
        rf("review", "review").r().i().doc("parent"),
        rf("plan", "plan").i().doc("the plan it is about"),
        strs("reviewers").i().doc("the reviewers who raised it"),
        en("severity", SEVERITY).r().i().doc("HIGH, MEDIUM or LOW"),
        str("text").r().i().doc("the concern"),
        bool("actionable")
            .i()
            .doc("a concern the planner can act on (convergence counts it)"),
        bool("drift")
            .i()
            .doc("a drift finding: advisory, never a blocker"),
        strs("evidence").i().doc("what the reviewers cited"),
        en("status", CONCERN_STATUS)
            .c()
            .doc("the outcome of the disposition naming it with the highest round, else open"),
    ],
)
.append_only()
.scoped_by("review");

const CONFLICT_STATUS: &[&str] = &["open", "resolved"];

/// A conflict a plan revision reports between a reviewer's ask and a property the plan must keep, named `CF1` and
/// scoped by its phase, in the phase's file: raised by `record_conflict`, closed by `resolve_conflict`.
pub const REVISION_CONFLICT: Kind = Kind::new(
    "revision_conflict",
    "A conflict a plan revision reported (GSD REVISION_CONFLICT lines)",
    &[
        rf("phase", "phase").r().i().doc("parent"),
        rf("plan", "plan")
            .r()
            .i()
            .doc("the plan whose revision reported it"),
        str("dimension")
            .r()
            .i()
            .doc("the checker dimension or review lane that asked"),
        str("property").r().i().doc("what the ask would change"),
        str("conflicts_with")
            .r()
            .i()
            .doc("the decision, requirement or must-have it would break"),
        str("alternatives")
            .i()
            .doc("the ways out the planner named"),
        ts("raised").h().doc("when `record_conflict` raised it"),
        block("resolved", RESOLVED)
            .h()
            .doc("when and how `resolve_conflict` closed it"),
        en("status", CONFLICT_STATUS)
            .c()
            .doc("resolved once `resolved` is set, open while only `raised` is, else none"),
    ],
)
.scoped_by("phase");
