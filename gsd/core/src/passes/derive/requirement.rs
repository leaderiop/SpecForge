//! The computed values of a requirement beyond its traceability `status` (`spec/project.md`): its `proof`, how far
//! its thread reaches, read once per requirement by `passes::thread::requirement_proof`.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::commits::Commit;
use crate::passes::plans::of_kind;
use crate::passes::thread::{Proof, requirement_proof};

/// The thread state of every requirement, by id: computed once per compile, for `proof` and for every reader of it.
pub(crate) fn proofs<'a>(input: &'a PassInput, commits: &[Commit]) -> BTreeMap<&'a str, Proof> {
    of_kind(input, "requirement")
        .map(|r| (r.id.as_str(), requirement_proof(input, commits, &r.id)))
        .collect()
}

/// `proof` of every requirement.
pub(crate) fn values(proofs: &BTreeMap<&str, Proof>) -> Vec<PassValue> {
    proofs
        .iter()
        .map(|(requirement, proof)| value(requirement, "proof", proof.as_str()))
        .collect()
}

#[cfg(test)]
mod tests;
