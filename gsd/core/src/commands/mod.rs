//! The commands of `@gsd/core` (K5): the writing commands the workflows call. r40 builds `record_verdict`; r41
//! builds the others, each file the commands of one family (D-r41-2).

mod complexity;
mod digest;
mod execute;
mod help;
mod milestone;
mod package_manager;
mod phase;
mod refactor;
mod reference;
pub(crate) mod reports;
pub mod smart_entry;
mod state;
pub(crate) mod support;
pub(crate) use support::natural;
mod surface;
mod verdict;
mod verify;
mod windows;
mod workstreams;

use specforge_extension_sdk::prelude::*;

/// The harness fields and the one thing that writes each: `(kind, field, writer)`. A writer is a writing command, or
/// the collector `gsd-git` for a commit's fields. `tests/writer_roster.rs` pins the table against the `H` fields the
/// kinds declare. `record_window` is the writer of a window's `recorded_at`; `propose_refactor` also stamps it on the
/// deviation window it creates under `refactor.trigger_strict`.
pub const WRITERS: &[(&str, &str, &str)] = &[
    ("milestone", "completion", "close_milestone"),
    ("checkpoint", "answer", "answer_checkpoint"),
    ("user_setup", "done", "confirm_setup"),
    ("summary", "started", "complete_plan"),
    ("summary", "completed", "complete_plan"),
    ("pause", "at", "pause"),
    ("pause", "by", "pause"),
    ("blocker", "raised", "raise_blocker"),
    ("uat_test", "result", "set_uat_result"),
    ("uat_test", "source", "set_uat_result"),
    ("validation", "approval", "approve_validation"),
    ("broken_window", "waiver", "waive_window"),
    ("broken_window", "fixed_at", "fix_window"),
    ("broken_window", "recorded_at", "record_window"),
    ("refactor_proposal", "score", "propose_refactor"),
    ("refactor_proposal", "baseline", "propose_refactor"),
    ("refactor_proposal", "delta", "propose_refactor"),
    ("refactor_proposal", "reasons", "propose_refactor"),
    ("refactor_proposal", "candidates", "propose_refactor"),
    ("refactor_proposal", "disposition", "dispose_refactor"),
    ("phase_spec", "context_digest", "record_spec"),
    ("research", "context_digest", "record_research"),
    ("validation", "context_digest", "record_validation"),
    ("commit", "hash", "gsd-git"),
    ("commit", "task", "gsd-git"),
    ("commit", "plan", "gsd-git"),
    ("commit", "type", "gsd-git"),
    ("commit", "subject", "gsd-git"),
    ("commit", "at", "gsd-git"),
    ("commit", "files", "gsd-git"),
    ("commit", "created", "gsd-git"),
    ("commit", "insertions", "gsd-git"),
    ("commit", "deletions", "gsd-git"),
    ("commit", "descends_from", "gsd-git"),
];

pub fn declare(c: &mut ContributionsBuilder) {
    verdict::declare(c);
    digest::declare(c);
    execute::declare(c);
    verify::declare(c);
    state::declare(c);
    windows::declare(c);
    refactor::declare(c);
    milestone::declare(c);
    phase::declare(c);
    workstreams::declare(c);
    help::declare(c);
    reports::declare(c);
    reference::declare(c);
    package_manager::declare(c);
    smart_entry::declare(c);
    surface::declare(c);
}
