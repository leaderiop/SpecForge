//! The kinds of execution (`spec/execute.md`): a plan's summary, its deliverables and its deviations.

use gsd_kit::dsl::*;

const SUMMARY_STATUS: &[&str] = &["complete", "halted"];
const REQUIRES: &[Field] = &[rf("phase", "phase").r(), str("provides").r()];
const ACTUALS: &[Field] = &[int("tasks"), int("commits")];

/// `NN-MM-SUMMARY.md`, named `03-02-SUMMARY` and scoped by its plan. It lives in the plan's own file, so the executors
/// of one wave never write the same file. GSD's `actuals.tokens` is absent until a harness exists (decision U4).
// Paths are `strs`, not `files`: a commit may delete or rename a path, so the host's E016 would fire on it.
pub const SUMMARY: Kind = Kind::new(
    "summary",
    "What a plan did when it ended (GSD NN-MM-SUMMARY.md)",
    &[
        rf("plan", "plan").r().i().doc("`phase:` + `plan:`"),
        en("status", SUMMARY_STATUS)
            .r()
            .doc("`status:`; a halted summary blocks the plans that depend on its plan"),
        str("one_liner").r().doc("the bold one-liner"),
        str("subsystem").r().doc("`subsystem:`"),
        strs("tags").doc("`tags:`"),
        blocks("requires", REQUIRES).doc("`requires:`"),
        strs("provides").doc("`provides:`"),
        strs("affects").doc("`affects:`"),
        strs("tech_added").doc("`tech-stack: added`"),
        strs("tech_patterns").doc("`tech-stack: patterns`"),
        rfs("key_decisions", "decision").doc("`key-decisions:`"),
        strs("patterns_established").doc("`patterns-established:`"),
        rfs("requirements_completed", "requirement")
            .r()
            .doc("`requirements-completed:`; gsd/E401 compares it with the plan's"),
        strs("threat_flags").doc("`## Threat Flags`"),
        str("next_readiness").doc("`## Next Phase Readiness`"),
        str("halted_because").doc("required when `status halted` (gsd/E402)"),
        ts("started")
            .h()
            .doc("written by the command that opens the plan, with the host's clock"),
        ts("completed")
            .h()
            .doc("`completed:`; written by the command that closes the plan"),
        str("duration")
            .c()
            .doc("`duration:` from `started` and `completed`: `45 min` or `1h 5m`"),
        block("actuals", ACTUALS)
            .c()
            .doc("`actuals:`: the plan's tasks and its recorded commits"),
        strs("files_created")
            .c()
            .doc("`key-files: created`, from the plan's commits"),
        strs("files_modified")
            .c()
            .doc("`key-files: modified`, from the plan's commits"),
    ],
)
.scoped_by("plan");

const KIND: &[&str] = &[
    "unit",
    "integration",
    "e2e",
    "automated_ui",
    "manual_procedural",
    "other",
];
const CHECK_STATUS: &[&str] = &["pass", "fail", "unknown"];
const CHECKS: &[Field] = &[
    en("kind", KIND).r(),
    str("ref").r(),
    en("status", CHECK_STATUS).c(),
];

/// A `coverage:` item of a summary, named `D1` and scoped by it. Each check's `status` is a computed member: the
/// derive pass reads it from the test report.
pub const DELIVERABLE: Kind = Kind::new(
    "deliverable",
    "A thing a plan delivers and how it is checked (GSD SUMMARY coverage)",
    &[
        rf("summary", "summary").r().i().doc("parent"),
        str("description").r().doc("`description:`"),
        rf("requirement", "requirement").doc("`requirement:`"),
        bool("human_judgment")
            .r()
            .doc("`human_judgment:`; false means the checks decide"),
        str("rationale").doc("`rationale:`; why a human judges it (gsd/W408)"),
        blocks("checks", CHECKS)
            .doc("`verification:`; a check's `ref` is `path#test name`, GSD's own test link"),
    ],
)
.scoped_by("summary");

const DEVIATION_KIND: &[&str] = &["deviation", "issue"];

/// A line of `## Deviations from Plan` or `## Issues Encountered`, named `DV1`: append-only, every field immutable.
pub const DEVIATION: Kind = Kind::new(
    "deviation",
    "A deviation from the plan, or an issue met while executing it (GSD SUMMARY)",
    &[
        rf("summary", "summary").r().i().doc("parent"),
        en("kind", DEVIATION_KIND)
            .r()
            .i()
            .doc("a deviation or an issue"),
        str("rule").i().doc("the executor's rule, `Rule 1 - Bug`"),
        str("text").r().i().doc("what happened"),
        strs("files").i().doc("the paths it touched"),
    ],
)
.append_only()
.scoped_by("summary");

const HANDOFF_STATE: &[&str] = &["in_progress", "blocked", "almost_done"];
const WORK_STATE: &[&str] = &["done", "in_progress", "not_started"];
const COMPLETED_WORK: &[Field] = &[rf("task", "task"), en("state", WORK_STATE)];
const RESOLVED: &[Field] = &[ts("at"), str("how")];

/// `/gsd-pause-work`'s record, named `PS1`: what a teammate who pulls the repository must see. Removed on resume.
pub const PAUSE: Kind = Kind::new(
    "pause",
    "Work paused for a handoff (GSD /gsd-pause-work)",
    &[
        rf("phase", "phase")
            .i()
            .doc("absent when the pause is not in a phase"),
        rf("plan", "plan").doc("the plan that was open"),
        ts("at")
            .h()
            .doc("written by the pausing command, with the host's clock"),
        str("by").h().doc("who paused"),
        str("reason").r().doc("why the work stopped"),
        rf("handoff", "handoff").doc("`.continue-here.md`"),
    ],
)
.scoped_by("phase");

/// `.continue-here.md`, named `HO1`.
pub const HANDOFF: Kind = Kind::new(
    "handoff",
    "What the next session needs to continue (GSD .continue-here.md)",
    &[
        rf("phase", "phase")
            .i()
            .doc("absent when the handoff is not in a phase"),
        rf("task", "task").doc("the task that was open"),
        en("state", HANDOFF_STATE).r().doc("where the work stands"),
        str("current_state").r().doc("`<current_state>`"),
        blocks("completed_work", COMPLETED_WORK).doc("`<completed_work>`"),
        strs("remaining_work").doc("`<remaining_work>`"),
        rfs("decisions_made", "decision").doc("`<decisions_made>`"),
        rfs("blockers", "blocker").doc("`<blockers>`"),
        str("context").doc("`<context>`"),
        str("next_action").r().doc("`<next_action>`"),
    ],
)
.scoped_by("phase");

/// STATE.md's `### Blockers/Concerns`, named `BL1`, in the project scope.
pub const BLOCKER: Kind = Kind::new(
    "blocker",
    "A blocker or concern that stops or slows the work (GSD STATE.md Blockers/Concerns)",
    &[
        str("text").r().doc("the blocker"),
        rf("phase", "phase").doc("the phase it blocks"),
        rf("plan", "plan").doc("the plan it blocks"),
        ts("raised")
            .h()
            .doc("written by the command that raises it, with the host's clock"),
        block("resolved", RESOLVED)
            .doc("how and when it was resolved (an authored time: gsd/E107)"),
    ],
);
