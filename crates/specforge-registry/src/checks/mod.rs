//! The host's checks over the entity snapshot's records (ADR 0019, ADR 0031):
//! the structural checks, in one order behind one gate, then the rule set.
//! Only [`RegistryBuild::check`] reaches them; no check reads a graph node.

mod fields;
mod identifiers;
mod index;
mod kinds;
mod references;
mod values;

use specforge_common::Diagnostic;

use crate::RegistryBuild;
use crate::entity::RuleInput;
use crate::rules::CustomVerdicts;

impl RegistryBuild {
    /// Every check over `input`, the entity snapshot's records, in this
    /// order:
    ///
    /// 1. unless [`Self::structural_only`]: E024 unknown kind, E013
    ///    reserved ID, E014 identifier length, W020 unknown field, E022
    ///    reference to the wrong kind, E061 value not of its declared type;
    /// 2. the rule set ([`crate::rules::Rules::check`]), asking `verdicts`
    ///    for `custom` rules.
    ///
    /// Each check reports its entities in record (id) order.
    pub fn check(&self, input: &RuleInput<'_>, verdicts: &dyn CustomVerdicts) -> Vec<Diagnostic> {
        let entities = input.entities;
        let mut diagnostics = Vec::new();
        if !self.structural_only() {
            diagnostics.extend(kinds::unknown(entities, &self.kinds)); // E024
            diagnostics.extend(identifiers::reserved(entities, &self.kinds)); // E013
            diagnostics.extend(identifiers::length(entities)); // E014
            diagnostics.extend(fields::unknown(entities, &self.kinds, &self.fields)); // W020
            diagnostics.extend(references::mistyped(entities, &self.kinds, &self.fields)); // E022
            diagnostics.extend(values::mistyped(entities, &self.kinds, &self.fields)); // E061
        }
        diagnostics.extend(self.rules.check(input, verdicts));
        diagnostics
    }

    /// No loaded extension declares an entity kind: the checks that read
    /// kinds, fields and identifiers do not run. With no extension loaded
    /// the environment says so (I002).
    pub fn structural_only(&self) -> bool {
        self.kinds.is_empty()
    }
}
