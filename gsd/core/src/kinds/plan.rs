//! `plan`, and the kinds scoped by it (`spec/plan.md`).
//!
//! Also ports gsd-core bin/lib modules: task-content-resolution.cjs.

use super::dsl::*;

const PLAN_TYPE: &[&str] = &["execute", "tdd"];
const PLAN_STATUS: &[&str] = &[
    "draft",
    "checked",
    "ready",
    "executing",
    "complete",
    "halted",
    "blocked",
];
const COUPLING: &[Field] = &[rf("peer", "plan").r(), str("reason").r()];
const ESTIMATE: &[Field] = &[int("tokens")];

/// `NN-MM-PLAN.md`, named `03-02` and scoped by its phase; a quick plan has no phase and is named by its quick id.
// Paths are `strs`, not `files`: a plan names files it will create, so the host's E016
// ("this file does not exist") would fire on every one.
pub const PLAN: Kind = Kind::new(
    "plan",
    "A plan: the work of one phase in tasks, with its must-haves (GSD NN-MM-PLAN.md)",
    &[
        rf("phase", "phase")
            .i()
            .doc("`phase:`; absent for a quick plan (gsd/W323 checks the name)"),
        en("type", PLAN_TYPE).r().doc("`type:`"),
        rfs("depends_on", "plan").doc("`depends_on:`"),
        strs("files_modified")
            .r()
            .doc("`files_modified:` (paths from the project root, not existence-checked)"),
        strs("files_deleted").doc("`files_deleted:`"),
        blocks("coupling_justified", COUPLING)
            .doc("`coupling_justified:`; exempts only non-file coupling"),
        rfs_any("gap_closure", &["verify_gap", "uat_gap"])
            .doc("`gap_closure: true` and `gap_ids:`; the gaps this plan closes"),
        rfs("requirements", "requirement")
            .doc("`requirements:` (non-empty: gsd/E302 is the one diagnostic of its absence)"),
        bool("wave0").doc("`wave: 0`: the plan builds the phase's wave-0 items"),
        str("agent_hint").doc("`agent_hint:`"),
        block("estimate", ESTIMATE).doc("`estimate:`"),
        str("objective").r().doc("`<objective>`"),
        str("purpose").doc("`Purpose:`"),
        str("output").doc("`Output:`"),
        strs("context_refs").doc("`<context>` @-refs (paths, not existence-checked)"),
        strs("verification").doc("`<verification>`"),
        strs("success_criteria").doc("`<success_criteria>`"),
        int("wave")
            .c()
            .doc("longest `depends_on` path plus the phase's base (phase-plan-index)"),
        bool("autonomous")
            .c()
            .doc("false exactly when the plan has a checkpoint"),
        en("status", PLAN_STATUS)
            .c()
            .doc("derived from the plan-check verdict and the summary"),
    ],
)
.scoped_by("phase");

const TASK_TYPE: &[&str] = &["auto", "tdd", "tracer", "manual"];
const RATING: &[&str] = &["reversible", "costly", "one_way"];
const REVERSIBILITY: &[Field] = &[
    en("rating", RATING).r(),
    str("why"),
    rf("settled_by", "decision"),
];
const VERIFY: &[Field] = &[str("automated").r(), str("fails_when")];
const TASK_STATUS: &[&str] = &["pending", "done"];

/// `<task type="auto|tdd">`, named `T1` within its plan; the order is the name's number.
// `commits` is not a field of the task: it is the inverse of `commit.task` (`commit`, round r38).
pub const TASK: Kind = Kind::new(
    "task",
    "A task of a plan (GSD <task>)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        en("type", TASK_TYPE).r().doc("`type=`"),
        strs("files").doc("`<files>` (paths, not existence-checked; required for `auto`, gsd/E303)"),
        strs("read_first").doc("`<read_first>`"),
        str("precondition").doc("`<precondition>`"),
        block("reversibility", REVERSIBILITY)
            .doc("`<reversibility>`; `settled_by` names the decision that already settles a one-way door (gsd/W308)"),
        str("action").r().doc("`<action>`"),
        blocks("verify", VERIFY).doc(
            "`<verify><automated>` and its `<fails_when>`: one item per command (gsd/E303, gsd/E507)",
        ),
        str("verify_text")
            .doc("a bare `<verify>` body: not an `<automated>` command, so no probe reads it"),
        str("verify_human").doc("`<verify><human-check>`: harvested into UAT"),
        strs("acceptance").doc("`<acceptance_criteria>`"),
        str("done").r().doc("`<done>`"),
        strs("verify_paths")
            .c()
            .doc("the directory and the manifest each `automated` command runs in (`cd`, `--prefix`), read by gsd/E320; each distinct path is listed once, on the first task (by id) that names it, so the host reads it once"),
        en("status", TASK_STATUS)
            .c()
            .doc("done when its plan's summary lists it complete"),
    ],
)
.scoped_by("plan");

const CHECKPOINT_KIND: &[&str] = &["human_verify", "decision", "human_action", "tdd_review"];
const GATE: &[&str] = &["blocking", "blocking_human", "advisory"];
const OPTION: &[Field] = &[str("name").r(), strs("pros"), strs("cons")];
const ANSWER: &[Field] = &[str("option"), str("text"), str("by"), ts("at")];

/// `<task type="checkpoint:*">`, named `C1` within its plan.
pub const CHECKPOINT: Kind = Kind::new(
    "checkpoint",
    "A checkpoint of a plan: a point where a human verifies, decides or acts (GSD <task type=\"checkpoint:*\">)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        rf("after", "task").doc("position in the task order"),
        en("kind", CHECKPOINT_KIND).r().doc("`type=`"),
        en("gate", GATE).doc("`gate=` (blocking when absent)"),
        str("what_built").doc("`<what-built>`: required for `human_verify` (gsd/E307)"),
        str("how_to_verify").doc("`<how-to-verify>`: required for `human_verify` (gsd/E307)"),
        str("decision").doc("`<decision>`: required for `decision` (gsd/E307)"),
        blocks("options", OPTION)
            .doc("`<options>`: required for `decision`; names unique within the checkpoint (gsd/E307)"),
        str("auto_select").doc("`auto_select=`: names one of `options` (gsd/E307)"),
        str("instructions").doc("`<instructions>`: required for `human_action` (gsd/E307)"),
        str("action").doc("`<action>`: required for `human_action` (gsd/E307)"),
        str("verification").doc("`<verification>`: required for `human_action` (gsd/E307)"),
        str("resume_signal").r().doc("`<resume-signal>`"),
        block("answer", ANSWER)
            .h()
            .doc("the human's reply, written by the command that resumes, with the host's clock"),
    ],
)
.scoped_by("plan");

const OUTCOME: &[&str] = &["addressed", "deferred"];

/// A row of the plan's Review Dispositions Ledger, named `DSP1`: append-only, every field immutable (GSD
/// `planner-reviews.md`; written by `/gsd-plan-review-convergence`).
pub const DISPOSITION: Kind = Kind::new(
    "disposition",
    "A review finding and what the plan did about it (GSD Review Dispositions Ledger)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        int("round").r().i().doc("the review round"),
        str("reviewer").r().i().doc("who raised it"),
        str("feedback").r().i().doc("the finding"),
        en("outcome", OUTCOME).r().i().doc("addressed or deferred"),
        str("reason").i().doc("why it was deferred"),
    ],
)
.append_only()
.scoped_by("plan");

const ENV_VAR: &[Field] = &[str("name").r(), str("source")];
const DASHBOARD: &[Field] = &[str("task"), str("location"), str("details")];
const SETUP_DONE: &[Field] = &[ts("at"), str("by")];

/// `user_setup:` of a plan, named `US1`: what a human must do outside the code.
pub const USER_SETUP: Kind = Kind::new(
    "user_setup",
    "A setup step only a human can do: an account, an environment variable, a dashboard setting (GSD user_setup)",
    &[
        rf("plan", "plan").r().i().doc("parent"),
        str("service").r().doc("`service:`"),
        str("why").r().doc("`why:`"),
        blocks("env_vars", ENV_VAR).doc("`env_vars:`"),
        blocks("dashboard", DASHBOARD).doc("`dashboard_config:`"),
        strs("local_dev").doc("`local_dev:`"),
        block("done", SETUP_DONE)
            .h()
            .doc("written when the human confirms (`confirm_setup`)"),
    ],
)
.scoped_by("plan");
