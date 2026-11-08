//! The order a collect runs its collectors in, and the facts each `facts` input is served (ADR 0345 D3): a collector
//! runs after the collectors that answer a kind it reads, and is served their admitted facts of it; a kind no
//! collector that ran answers is served from the recorded stores.

use std::collections::BTreeMap;
use std::path::Path;

use specforge_common::codes;
use specforge_protocol_types::{Fact, Inputs};
use specforge_registry::collectors::Edges;

use super::{Collector, fail};
use crate::{Containment, OpError, OpErrorKind};

/// `selected` in the order their facts inputs give (declaration order otherwise); E119 naming the collectors of a
/// cycle.
pub(super) fn ordered(selected: Vec<&Collector>) -> Result<Vec<&Collector>, OpError> {
    let edges: Vec<Edges<'_>> = selected
        .iter()
        .map(|collector| Edges {
            answers: &collector.answers,
            inputs: &collector.inputs,
        })
        .collect();
    match specforge_registry::collectors::order(&edges) {
        Ok(order) => Ok(order.into_iter().map(|at| selected[at]).collect()),
        Err(on_cycle) => {
            let names: Vec<String> = on_cycle
                .into_iter()
                .map(|at| selected[at].name.clone())
                .collect();
            let cycle = specforge_registry::collectors::cycle(&names);
            Err(fail(
                OpErrorKind::PreconditionFailed,
                codes::E119,
                cycle.message,
            ))
        }
    }
}

/// What the collectors that ran so far answered, by kind, and where the recorded stores are read.
pub(super) struct Served<'a> {
    root: &'a Path,
    containment: &'a Containment,
    answered: BTreeMap<String, Vec<Fact>>,
}

impl<'a> Served<'a> {
    pub(super) fn new(root: &'a Path, containment: &'a Containment) -> Self {
        Served {
            root,
            containment,
            answered: BTreeMap::new(),
        }
    }

    /// What `collector`'s `facts` input is served, sorted by kind and id: for each kind, the admitted facts of the
    /// collectors that ran and answer it, else the recorded stores' (read inside the containment).
    pub(super) fn facts(&self, collector: &Collector) -> Vec<Fact> {
        let kinds = Inputs(&collector.inputs).facts();
        let (here, recorded): (Vec<&str>, Vec<&str>) = kinds
            .into_iter()
            .partition(|kind| self.answered.contains_key(*kind));
        let mut facts: Vec<Fact> = here
            .iter()
            .flat_map(|kind| self.answered[*kind].iter().cloned())
            .collect();
        if !recorded.is_empty() {
            facts.extend(specforge_project::facts::recorded_facts(
                &self.containment.disk(),
                self.root,
                &recorded,
            ));
        }
        facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        facts
    }

    /// `collector` ran and the host admitted `facts` of it: each kind it answers is served from them now, none
    /// when they were refused.
    pub(super) fn answered(&mut self, collector: &Collector, facts: &[Fact]) {
        for kind in &collector.answers {
            let of: Vec<Fact> = facts.iter().filter(|f| &f.kind == kind).cloned().collect();
            self.answered.entry(kind.clone()).or_default().extend(of);
        }
    }
}
