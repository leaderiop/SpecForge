//! The order a collect runs its collectors in, and the facts each `facts` input is served (ADR 0345 D3): a collector
//! runs after the collectors that answer a kind it reads, and is served their admitted facts of it; a kind no
//! collector that ran answers is served from the recorded stores, and W201 names the collectors that answer a kind
//! neither serves a fact of.

use std::collections::BTreeMap;
use std::path::Path;

use specforge_common::{Diagnostic, codes};
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

/// What the collectors that ran so far answered, by kind, where the recorded stores are read, and the collectors the
/// project declares (the ones W201 names).
pub(super) struct Served<'a> {
    root: &'a Path,
    containment: &'a Containment,
    declared: &'a [Collector],
    answered: BTreeMap<String, Vec<Fact>>,
}

impl<'a> Served<'a> {
    pub(super) fn new(
        root: &'a Path,
        containment: &'a Containment,
        declared: &'a [Collector],
    ) -> Self {
        Served {
            root,
            containment,
            declared,
            answered: BTreeMap::new(),
        }
    }

    /// What `collector`'s `facts` input is served, sorted by kind and id: for each kind, the admitted facts of the
    /// collectors that ran and answer it, else the recorded stores' (read inside the containment). A kind served
    /// from the stores that none of them holds a fact of is W201 in `diagnostics`, naming the declared collectors
    /// that answer it: the collector reads nothing of it (ADR 0345 D3).
    pub(super) fn facts(
        &self,
        collector: &Collector,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<Fact> {
        let kinds = Inputs(&collector.inputs).facts();
        let (here, recorded): (Vec<&str>, Vec<&str>) = kinds
            .into_iter()
            .partition(|kind| self.answered.contains_key(*kind));
        let mut facts: Vec<Fact> = here
            .iter()
            .flat_map(|kind| self.answered[*kind].iter().cloned())
            .collect();
        if !recorded.is_empty() {
            let stored = specforge_project::facts::recorded_facts(
                &self.containment.disk(),
                self.root,
                &recorded,
            );
            for kind in recorded
                .iter()
                .filter(|kind| !stored.iter().any(|f| &f.kind == *kind))
            {
                diagnostics.push(self.unserved(collector, kind));
            }
            facts.extend(stored);
        }
        facts.sort_by(|a, b| (&a.kind, &a.id).cmp(&(&b.kind, &b.id)));
        facts
    }

    /// W201: `collector` reads `kind` facts, and no collector of this collect answered them nor does a recorded
    /// store hold one; the message names the declared collectors that answer the kind, from their declarations.
    fn unserved(&self, collector: &Collector, kind: &str) -> Diagnostic {
        let answering: Vec<&str> = self
            .declared
            .iter()
            .filter(|c| c.answers.iter().any(|answer| answer == kind))
            .map(|c| c.name.as_str())
            .collect();
        let read = format!(
            "'{}' reads '{kind}' facts, and none was served: no collector of this collect answers them and no recorded \
             store holds one",
            collector.name
        );
        if answering.is_empty() {
            return Diagnostic::new(
                codes::W201,
                format!("{read}; no loaded collector answers them"),
            )
            .with_suggestion("load the extension whose collector answers them, then collect");
        }
        let names: Vec<String> = answering.iter().map(|name| format!("'{name}'")).collect();
        let runners: Vec<String> = answering
            .iter()
            .map(|name| {
                specforge_common::remedy::command("`specforge collect --runner {}`", &[name])
            })
            .collect();
        Diagnostic::new(
            codes::W201,
            format!("{read}; {} answer them", names.join(", ")),
        )
        .with_suggestion(format!(
            "collect with them first ({}), or collect every detected collector (`specforge collect`)",
            runners.join(", ")
        ))
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
