//! `plan`, and the kinds scoped by it (`spec/plan.md`).

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
// `gap_closure → verify_gap | uat_gap` is declared with those kinds (round r39): a reference to a kind no loaded
// extension declares is W021. Paths are `strs`, not `files`: a plan names files it will create, so the host's E016
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
// `commits → commit` is declared with `commit` (round r38); `verify_paths` with gsd/E320.
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
