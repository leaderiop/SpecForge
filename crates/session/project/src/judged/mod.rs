//! Judged rules over a project (ADR 0290): the digests a verdict is fresh against, the state of each judged rule on
//! each entity, and the inputs a verdict is recorded with. The declaration is the registry's
//! (`specforge_registry::judged`).
//!
//! A judged rule fires on an entity of a target kind while its verdict is missing, unreadable, not recorded in the
//! harness ledger, stale (an input digests differently from when it was recorded) or failing.

mod digest;
mod inputs;
mod memo;
mod report;
mod select;
mod state;

use std::collections::BTreeMap;

pub use digest::{entity_digest, file_digest};
pub use inputs::{Cited, Inputs, inputs};
pub use memo::DigestMemo;
pub use state::{Evaluated, evaluate};

use specforge_protocol_types::Judgement;

/// Every judged rule's state on every entity it judges, by entity path and the rule's qualified code.
#[derive(Debug, Clone, Default)]
pub struct Judgements(BTreeMap<(String, String), Judgement>);

impl Judgements {
    /// The state of `rule` (qualified code) on `entity`.
    pub fn get(&self, entity: &str, rule: &str) -> Option<&Judgement> {
        self.0.get(&(entity.to_string(), rule.to_string()))
    }

    /// Every rule's state on `entity`, in the order of the rules' codes.
    pub fn of<'a>(&'a self, entity: &'a str) -> impl Iterator<Item = &'a Judgement> {
        self.0
            .range((entity.to_string(), String::new())..)
            .take_while(move |((e, _), _)| e == entity)
            .map(|(_, judgement)| judgement)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn insert(&mut self, entity: &str, judgement: Judgement) {
        self.0
            .insert((entity.to_string(), judgement.rule.clone()), judgement);
    }
}
