//! The kinds of `@gsd/core`, as tables (`dsl`): one file per area of the model.

mod commit;
mod discuss;
mod dsl;
mod execute;
mod learning;
mod must_haves;
mod plan;
mod project;
mod research;
mod roadmap;
mod spec;
mod uat;
mod verify;

use dsl::Kind;
use specforge_extension_sdk::prelude::*;

/// Every kind declared so far, in the order of the model's files.
const KINDS: &[&Kind] = &[
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
    &plan::USER_SETUP,
    &execute::SUMMARY,
    &execute::DELIVERABLE,
    &execute::DEVIATION,
    &commit::COMMIT,
    &execute::PAUSE,
    &execute::HANDOFF,
    &execute::BLOCKER,
    &verify::VERIFICATION,
    &verify::VERIFY_GAP,
    &verify::OVERRIDE,
    &verify::HUMAN_ITEM,
    &uat::UAT_SESSION,
    &uat::UAT_TEST,
    &uat::UAT_GAP,
];

pub fn declare(c: &mut ContributionsBuilder) {
    dsl::declare(c, KINDS);
}

/// `kind.field` of every field whose structured value the digest reads (see `dsl::structured`).
pub fn structured() -> Vec<String> {
    dsl::structured(KINDS)
}

/// `kind.field` of every computed field: what the derive pass answers.
pub fn computed() -> Vec<String> {
    dsl::computed(KINDS)
}
