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
