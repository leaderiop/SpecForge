//! The kinds of execution (`spec/execute.md`): a plan's summary, its deliverables and its deviations.

use super::dsl::*;

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
