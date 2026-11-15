//! The computed values of a requirement beyond its traceability `status` (`spec/project.md`): its `proof`, how far
//! its thread reaches, read once per requirement by `passes::thread::requirement_proof`, and its `satisfied_by`, the
//! code items the tasks of its plans name.
//!
//! `satisfied_by` reads each task's `items` as written, as `gsd/W522` does: an item a pending task plans to write
//! belongs to the requirement before any collect holds it, and this pass is sent no item facts (ADR 0353).

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::commits::Commit;
use crate::passes::plans::{of_kind, strings};
use crate::passes::thread::{Proof, requirement_proof, requirement_tasks};

/// The thread state of every requirement, by id: computed once per compile, for `proof` and for every reader of it.
pub(crate) fn proofs<'a>(input: &'a PassInput, commits: &[Commit]) -> BTreeMap<&'a str, Proof> {
    of_kind(input, "requirement")
        .map(|r| (r.id.as_str(), requirement_proof(input, commits, &r.id)))
        .collect()
}

/// `proof` of every requirement, and its `satisfied_by` when a task of its plans, of any type, names an item.
pub(crate) fn values(input: &PassInput, proofs: &BTreeMap<&str, Proof>) -> Vec<PassValue> {
    let mut out = Vec::new();
    for (requirement, proof) in proofs {
        out.push(value(requirement, "proof", proof.as_str()));
        let items: BTreeSet<String> = requirement_tasks(input, requirement)
            .into_iter()
            .flat_map(|task| strings(task, "items"))
            .collect();
        if !items.is_empty() {
            out.push(value(requirement, "satisfied_by", Vec::from_iter(items)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
