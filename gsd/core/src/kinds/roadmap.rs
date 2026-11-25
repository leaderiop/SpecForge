//! `milestone`, `phase` and `workstream` (`spec/project.md`).

use gsd_kit::dsl::*;

const ABANDONED: &[Field] = &[str("reason").r(), str("ruled_by").r(), date("at").r()];
const COMPLETION: &[Field] = &[date("at"), rf("audit", "milestone_audit")];
const STATS: &[Field] = &[
    int("files"),
    int("loc"),
    int("phases"),
    int("plans"),
    int("tasks"),
    int("days"),
];
const PROGRESS: &[Field] = &[
    int("phases"),
    int("phases_complete"),
    int("plans"),
    int("plans_complete"),
    int("percent"),
];
const OFF_PACKAGE: &[Field] = &[str("name"), str("version")];
const RELEASE: &[Field] = &[
    str("tag"),
    str("commit"),
    blocks("packages", OFF_PACKAGE),
    int("more"),
];
const TAG: &[Field] = &[str("name"), str("commit")];
const MILESTONE_STATE: &[&str] = &["planned", "in_progress", "shipped", "abandoned"];

/// A milestone section of `ROADMAP.md`, named `v1.0` and scoped by its workstream when it has one. Its title is its
/// name (`MVP`): it declares no field for it (ADR 0387).
pub const MILESTONE: Kind = Kind::new(
    "milestone",
    "A milestone: a shippable version of the project (GSD ROADMAP.md, MILESTONES.md)",
    &[
        str("delivered").doc("Delivered"),
        strs("accomplishments").doc("Key accomplishments"),
        str("whats_next").doc("What's next"),
        str("goal").doc("**Goal:** of the Current Milestone in PROJECT.md"),
        strs("target_features").doc("**Target features:** of the Current Milestone in PROJECT.md"),
        block("abandoned", ABANDONED).doc("gsd-path's abandon ruling"),
        rf("workstream", "workstream")
            .i()
            .doc(
                "the workstream that scopes it; absent outside one (upstream: the workstream directory that holds \
                 its state)",
            ),
        block("completion", COMPLETION)
            .h()
            .doc("written by close_milestone (/gsd-complete-milestone) once its audit passes"),
        en("state", MILESTONE_STATE)
            .c()
            .doc("planned, in progress or shipped, from its phases and its completion"),
        str("first_commit")
            .c()
            .doc("its first recorded commit, oldest by author time; absent without one"),
        date("shipped").c().doc("Shipped: the completion's date"),
        block("stats", STATS)
            .c()
            .doc("Stats: from the graph and the recorded commits"),
        str("git_range")
            .c()
            .doc("Git range: from the recorded commits"),
        block("progress", PROGRESS)
            .c()
            .doc("state json's `progress`: its phases and plans, and the smaller of the two fractions as a percent"),
        str("branch")
            .c()
            .doc("the branch it is executed on: git.branching_strategy and its template"),
        strs("api_breaks").c().doc(
            "the API items the last collect found changed, narrowed or removed against the committed accepted \
             report: `<change> <id>`; absent when no API fact carries a change; gsd/E533 reads it",
        ),
        block("release", RELEASE).c().doc(
            "the release evidence gsd/E535 reads: `tag` and `commit` the recorded tag of a shipped milestone, \
             `packages` the `name` and `version` of each package fact not at an open milestone's version, at most 50, \
             and `more` the count past them; absent when nothing was observed",
        ),
        block("tag", TAG).h().doc(
            "written by gsd-git from `refs/tags/v<version>` for a shipped milestone named by its version: the tag's \
             `name`, and the `commit` it resolves to, absent when it resolves to none",
        ),
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

/// `### Phase N: Name`, named `03`, `03.1` (inserted) or `999.1` (backlog), scoped by its milestone. `Name` is its
/// title and `N` its name: its label is `Phase {name}: {title}`, so it declares no field for either (ADR 0387).
pub const PHASE: Kind = Kind::new(
    "phase",
    "A phase of a milestone, named 03, 03.1 (inserted) or 999.1 (backlog) (GSD ROADMAP.md)",
    &[
        rf("milestone", "milestone")
            .r()
            .i()
            .doc("the section it sits in"),
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
        bool("inserted").c().doc("(INSERTED): a decimal name"),
        bool("backlog").c().doc("a 999.x name"),
        str("branch")
            .c()
            .doc("the branch it is executed on: git.branching_strategy and its template"),
        bool("frontend").c().doc("the ui gate's frontend detection"),
        en("status", PHASE_STATUS)
            .c()
            .doc("the GSD phase-status ladder"),
        str("plans_complete")
            .c()
            .doc("Progress: Plans Complete, N/M"),
        date("completed").c().doc("Progress: Completed"),
        strs("critical_path").c().doc(
            "the task ids of the phase's heaviest chain of plans, in execution order: the chain follows `depends_on` \
             and shared paths (the strict waves), each plan weighing its `estimate.tokens`, else its task count; of \
             two chains of equal weight the longer wins, then the smaller plan id as a person reads it (`01-2` \
             before `01-10`); absent when no plan has a strict wave or the chain names no task",
        ),
        int("critical_weight").c().doc(
            "the weight of that chain: tokens and task counts summed as they come, so a phase with and without \
             estimates mixes units",
        ),
    ],
)
.scoped_by("milestone")
.complete_when("status", &["executed", "needs_review", "complete"]);

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
