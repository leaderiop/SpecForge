//! Judged rules over a project (ADR 0290): the digests a verdict is fresh against, the state of each judged rule on
//! each entity, and the inputs a verdict is recorded with. The declaration is the registry's
//! (`specforge_registry::judged`).
//!
//! A judged rule fires on an entity of a target kind while its verdict is missing, unreadable, not recorded in the
//! harness ledger, stale (an input digests differently from when it was recorded) or failing.

mod describe;
mod digest;
mod inputs;
mod memo;
mod select;
mod state;

use std::collections::BTreeMap;

pub use describe::{Described, InputsRefusal, describe};
pub use digest::{entity_digest, file_digest};
pub use inputs::{Cited, CitedReport, InputReport, Inputs, inputs};
pub use memo::DigestMemo;
pub use state::{Evaluated, evaluate};

use specforge_common::Diagnostic;
use specforge_protocol_types::Judgement;

/// Every judged rule's state on every entity it judges, by entity path and the rule's qualified code.
#[derive(Debug, Clone, Default)]
pub struct Judgements {
    states: BTreeMap<(String, String), Judgement>,
    /// Every finding the judged evaluation made (ADR 0302): the only diagnostics a write is not refused for.
    findings: Vec<Diagnostic>,
}

impl Judgements {
    /// The state of `rule` (qualified code) on `entity`.
    pub fn get(&self, entity: &str, rule: &str) -> Option<&Judgement> {
        self.states.get(&(entity.to_string(), rule.to_string()))
    }

    /// Every rule's state on `entity`, in the order of the rules' codes.
    pub fn of<'a>(&'a self, entity: &'a str) -> impl Iterator<Item = &'a Judgement> {
        self.states
            .range((entity.to_string(), String::new())..)
            .take_while(move |((e, _), _)| e == entity)
            .map(|(_, judgement)| judgement)
    }

    /// Every entity's judgements, by entity path: what an export lists.
    pub fn listed(&self) -> BTreeMap<String, Vec<Judgement>> {
        let mut listed: BTreeMap<String, Vec<Judgement>> = BTreeMap::new();
        for ((entity, _), judgement) in &self.states {
            listed
                .entry(entity.clone())
                .or_default()
                .push(judgement.clone());
        }
        listed
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub(crate) fn insert(&mut self, entity: &str, judgement: Judgement) {
        self.states
            .insert((entity.to_string(), judgement.rule.clone()), judgement);
    }

    pub(crate) fn found(&mut self, finding: Diagnostic) {
        self.findings.push(finding);
    }

    /// Whether `d` is a finding the judged evaluation made (ADR 0302): the same code, message, origin, subject and file
    /// (not the severity: a preset may raise it). A pass or guard that reuses a judged rule's code is not one.
    pub fn is_finding(&self, d: &Diagnostic) -> bool {
        let file = |d: &Diagnostic| d.span.as_ref().map(|s| s.file.clone());
        self.findings.iter().any(|f| {
            f.code == d.code
                && f.message == d.message
                && f.origin() == d.origin()
                && f.subject() == d.subject()
                && file(f) == file(d)
        })
    }
}
