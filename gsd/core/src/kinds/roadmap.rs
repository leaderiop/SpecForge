//! `milestone`, `phase` and `workstream` (`spec/project.md`).

use super::dsl::*;

const ABANDONED: &[Field] = &[str("reason").r(), str("ruled_by").r(), date("at").r()];
// `audit` (a reference to `milestone_audit`) joins with that kind in round r39.
const COMPLETION: &[Field] = &[date("at")];
const STATS: &[Field] = &[
    int("files"),
    int("loc"),
    int("phases"),
    int("plans"),
    int("tasks"),
    int("days"),
];
const MILESTONE_STATE: &[&str] = &["planned", "in_progress", "shipped", "abandoned"];

/// A milestone section of `ROADMAP.md`, named `v1.0` and scoped by its workstream when it has one.
pub const MILESTONE: Kind = Kind::new(
    "milestone",
    "A milestone: a shippable version of the project (GSD ROADMAP.md, MILESTONES.md)",
    &[
        str("name").r().doc("MVP"),
        str("delivered").doc("Delivered"),
        strs("accomplishments").doc("Key accomplishments"),
        str("whats_next").doc("What's next"),
        block("abandoned", ABANDONED).doc("gsd-path's abandon ruling"),
        rf("workstream", "workstream")
            .i()
            .doc("the workstream that scopes it; absent outside one"),
        block("completion", COMPLETION)
            .h()
            .doc("written by /gsd-complete-milestone once its audit passes"),
        en("state", MILESTONE_STATE)
            .c()
            .doc("planned, in progress or shipped, from its phases and its completion"),
        date("shipped").c().doc("Shipped: the completion's date"),
        block("stats", STATS)
            .c()
            .doc("Stats: from the graph and the recorded commits"),
        str("git_range")
            .c()
            .doc("Git range: from the recorded commits"),
    ],
)
.scoped_by("workstream");

const MODES: &[&str] = &["standard", "mvp"];
const PHASE_STATUS: &[&str] = &[
    "not_started",
    "planned",
    "in_progress",
    "executed",
    "needs_review",
    "complete",
    "deferred",
];

/// `### Phase N: Name`, named `03`, `03.1` (inserted) or `999.1` (backlog), scoped by its milestone.
pub const PHASE: Kind = Kind::new(
    "phase",
    "A phase of a milestone, named 03, 03.1 (inserted) or 999.1 (backlog) (GSD ROADMAP.md)",
    &[
        rf("milestone", "milestone")
            .r()
            .i()
            .doc("the section it sits in"),
        str("name").r().doc("heading text"),
        str("slug").r().doc("directory NN-slug"),
        str("goal").r().doc("Goal"),
        rfs("depends_on", "phase").doc("Depends on"),
        rfs("requirements", "requirement")
            .inverse("phase")
            .doc("Requirements; the requirement reads its phase back as `phase`"),
        strs("success_criteria")
            .r()
            .doc("Success Criteria (2 to 5, gsd/W103)"),
        en("mode", MODES).doc("Mode (/gsd-mvp-phase)"),
        str("tag").doc("the parenthetical, (Cluster B)"),
        bool("inserted").c().doc("(INSERTED): a decimal name"),
        bool("backlog").c().doc("a 999.x name"),
        bool("frontend").c().doc("the ui gate's frontend detection"),
        en("status", PHASE_STATUS)
            .c()
            .doc("the GSD phase-status ladder"),
        str("plans_complete")
            .c()
            .doc("Progress: Plans Complete, N/M"),
        date("completed").c().doc("Progress: Completed"),
    ],
)
.scoped_by("milestone");

const WORKSTREAM_STATUS: &[&str] = &["active", "complete"];

/// `.planning/workstreams/<name>/`: a scope for its milestones, phases and plans.
pub const WORKSTREAM: Kind = Kind::new(
    "workstream",
    "A workstream: a scope with its own milestones and phases (GSD workstreams)",
    &[
        str("purpose").r().doc("the workstream's description"),
        en("status", WORKSTREAM_STATUS)
            .c()
            .doc("complete when all its milestones shipped"),
    ],
);
