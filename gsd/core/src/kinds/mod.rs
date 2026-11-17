//! The kinds of `@gsd/core`, as tables (`dsl`): one file per area of the model.
//!
//! Also ports gsd-core bin/lib modules: artifacts.cjs.

mod audit;
mod commit;
mod discuss;
mod execute;
mod governance;
mod learning;
mod must_haves;
mod plan;
mod project;
mod research;
mod review;
mod roadmap;
mod spec;
mod uat;
mod validation;
mod verify;
mod windows;

pub use governance::INVARIANT_IN_FORCE;
use gsd_kit::dsl::{self, Kind};
pub use project::PROOF_STATES;
pub use roadmap::PHASE_COMPLETE;
use specforge_extension_sdk::prelude::*;

/// An option weighed: a checkpoint's options (GSD `<options>`).
const OPTION: &[dsl::Field] = &[
    dsl::str("name").r(),
    dsl::strs("pros").prose(),
    dsl::strs("cons").prose(),
];

/// Every kind declared so far, in the order of the model's files.
pub(crate) const KINDS: &[&Kind] = &[
    &project::PROJECT,
    &project::REQUIREMENT,
    &roadmap::MILESTONE,
    &roadmap::PHASE,
    &roadmap::WORKSTREAM,
    &learning::RETROSPECTIVE,
    &learning::LESSON,
    &learning::METHODOLOGY,
    &discuss::CONTEXT,
    &discuss::DECISION,
    &discuss::DEFERRED_IDEA,
    &spec::PHASE_SPEC,
    &spec::LOCKED_REQ,
    &spec::INTERVIEW,
    &spec::EDGE,
    &spec::SPEC_PROHIBITION,
    &discuss::DISCUSSION,
    &research::RESEARCH,
    &research::FINDING,
    &research::STACK_CHOICE,
    &research::PATTERN,
    &research::PITFALL,
    &research::DONT_HAND_ROLL,
    &research::OPEN_QUESTION,
    &research::SOURCE,
    &research::PACKAGE_AUDIT,
    &plan::PLAN,
    &plan::TASK,
    &plan::CHECKPOINT,
    &must_haves::TRUTH,
    &must_haves::ARTIFACT,
    &must_haves::KEY_LINK,
    &must_haves::PROHIBITION,
    &plan::DISPOSITION,
    &review::REVIEW,
    &review::REVIEW_CONCERN,
    &review::REVISION_CONFLICT,
    &plan::USER_SETUP,
    &execute::PLAN_START,
    &execute::SUMMARY,
    &execute::DELIVERABLE,
    &execute::DEVIATION,
    &commit::COMMIT,
    &execute::PAUSE,
    &execute::HANDOFF,
    &execute::BLOCKER,
    &execute::QUICK_BATCH,
    &verify::VERIFICATION,
    &verify::VERIFY_GAP,
    &verify::OVERRIDE,
    &verify::HUMAN_ITEM,
    &uat::UAT_SESSION,
    &uat::UAT_TEST,
    &uat::UAT_GAP,
    &validation::VALIDATION,
    &validation::VALIDATION_ROW,
    &validation::WAVE0_ITEM,
    &validation::MANUAL_CHECK,
    &audit::MILESTONE_AUDIT,
    &windows::BROKEN_WINDOW,
    &windows::REFACTOR_PROPOSAL,
    &governance::INVARIANT,
];

pub fn declare(c: &mut ContributionsBuilder) {
    dsl::declare(c, KINDS);
}

/// Every path field and block member path, with the mode its file check follows and its writer (see
/// `dsl::path_fields`).
pub fn path_fields() -> Vec<dsl::PathField> {
    dsl::path_fields(KINDS)
}

/// `kind.field` of every computed field: what the derive pass answers.
pub fn computed() -> Vec<String> {
    dsl::computed(KINDS)
}
