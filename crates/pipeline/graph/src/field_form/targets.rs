//! The kinds a reference field's ids may name (ADR 0117, ADR 0285): one, or a union.

use std::collections::BTreeSet;

use super::{FieldForms, Reading};

/// The kinds a reference field or member may name, in declared order, each loaded (a loaded extension declares it)
/// or absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targets<'f> {
    kinds: Vec<(&'f str, bool)>,
}

impl<'f> Targets<'f> {
    pub(super) fn new(kinds: &'f [String], loaded: &BTreeSet<String>) -> Option<Self> {
        (!kinds.is_empty()).then(|| Self {
            kinds: kinds
                .iter()
                .map(|kind| (kind.as_str(), loaded.contains(kind)))
                .collect(),
        })
    }

    /// Every kind it names, in declared order.
    pub fn kinds(&self) -> impl Iterator<Item = &'f str> + '_ {
        self.kinds.iter().map(|(kind, _)| *kind)
    }

    /// The kinds a loaded extension declares.
    pub fn loaded(&self) -> Vec<&'f str> {
        self.of(true)
    }

    /// The kinds no loaded extension declares.
    pub fn absent(&self) -> Vec<&'f str> {
        self.of(false)
    }

    fn of(&self, loaded: bool) -> Vec<&'f str> {
        self.kinds
            .iter()
            .filter(|(_, is)| *is == loaded)
            .map(|(kind, _)| *kind)
            .collect()
    }

    /// The kind, when it names exactly one.
    pub fn only(&self) -> Option<&'f str> {
        match self.kinds.as_slice() {
            [(kind, _)] => Some(kind),
            _ => None,
        }
    }

    /// The kind, when it names exactly one and a loaded extension declares it: what a scope field needs.
    pub fn only_loaded(&self) -> Option<&'f str> {
        match self.kinds.as_slice() {
            [(kind, true)] => Some(kind),
            _ => None,
        }
    }

    /// Whether an entity of `kind` is one this takes: any kind it names, loaded or not.
    pub fn admits(&self, kind: &str) -> bool {
        self.kinds.iter().any(|(named, _)| *named == kind)
    }

    /// The kinds as a message names them: `plan`, or `verify_gap or uat_gap`.
    pub fn text(&self) -> String {
        self.kinds().collect::<Vec<_>>().join(" or ")
    }
}

impl FieldForms {
    /// The kinds `kind`'s field or member `path` (a declared member path) may name, and which a loaded extension
    /// declares. `None` for a field with no declared target, a member of an open-fields kind, or an undeclared field.
    pub fn target(&self, kind: &str, path: &str) -> Option<Targets<'_>> {
        let Reading::Declared(form) = self.reading_at(kind, path) else {
            return None;
        };
        Targets::new(form.target_kinds(), &self.kinds)
    }

    /// Every declared (kind, member path) with a target, members included, in (kind, path) order.
    pub fn targets(&self) -> Vec<((&str, String), Targets<'_>)> {
        let mut targets = Vec::new();
        for ((kind, field), form) in self.iter() {
            form.collect_targets(kind, field.to_string(), &self.kinds, &mut targets);
        }
        targets.sort_by(|a, b| a.0.cmp(&b.0));
        targets
    }
}

impl super::FieldForm {
    fn collect_targets<'f>(
        &'f self,
        kind: &'f str,
        path: String,
        loaded: &BTreeSet<String>,
        out: &mut Vec<((&'f str, String), Targets<'f>)>,
    ) {
        if let Some(targets) = Targets::new(self.target_kinds(), loaded) {
            out.push(((kind, path.clone()), targets));
        }
        for (name, member) in self.members().into_iter().flat_map(|m| m.iter()) {
            member.collect_targets(kind, format!("{path}.{name}"), loaded, out);
        }
    }
}
