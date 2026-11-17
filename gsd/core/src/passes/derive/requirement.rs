//! The computed values of a requirement beyond its traceability `status` (`spec/project.md`): its `proof`, how far
//! its thread reaches, read once per requirement by `passes::thread::requirement_proof`, and its `satisfied_by`, the
//! code items the recorded commits of its tasks changed (ADR 0371 D6).
//!
//! `satisfied_by` reads each commit's `items_changed` as `gsd-git` wrote it (ADR 0352 D1): a commit whose field is
//! unwritten (no item facts were served, or a file's lines were unknown) adds nothing, so the value is absent until a
//! commit of one of its tasks is collected with item facts. The route's `item` step reads the same field, so the items
//! `specforge trace` prints and `satisfied_by` agree.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::commits::Commit;
use crate::passes::plans::of_kind;
use crate::passes::thread::{Proof, requirement_proof, requirement_tasks};

/// The thread state of every requirement, by id: computed once per compile, for `proof` and for every reader of it.
pub(crate) fn proofs<'a>(input: &'a PassInput, commits: &[Commit]) -> BTreeMap<&'a str, Proof> {
    of_kind(input, "requirement")
        .map(|r| (r.id.as_str(), requirement_proof(input, commits, &r.id)))
        .collect()
}

/// `proof` of every requirement, and its `satisfied_by` when a recorded commit of a task of its plans, of any type,
/// changed an item: sorted, each once.
pub(crate) fn values(
    input: &PassInput,
    commits: &[Commit],
    proofs: &BTreeMap<&str, Proof>,
) -> Vec<PassValue> {
    let mut out = Vec::new();
    for (requirement, proof) in proofs {
        out.push(value(requirement, "proof", proof.as_str()));
        let tasks: BTreeSet<&str> = requirement_tasks(input, requirement)
            .into_iter()
            .map(|task| task.id.as_str())
            .collect();
        let items: BTreeSet<String> = commits
            .iter()
            .filter(|c| c.task.as_deref().is_some_and(|t| tasks.contains(t)))
            .filter_map(|c| c.items_changed.clone())
            .flatten()
            .collect();
        if !items.is_empty() {
            out.push(value(requirement, "satisfied_by", Vec::from_iter(items)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
