//! The builders of a judged rule (ADR 0290): its outcomes, the kinds it targets and what it reads of each, and the
//! owner state a target waits for (ADR 0362). `RuleBuilder::judged` opens one.

use specforge_protocol_types::{JudgedDescriptor, JudgedInput, JudgedTarget, OwnerState};

/// Builder for [`JudgedDescriptor`].
pub struct JudgedBuilder(pub(crate) JudgedDescriptor);

impl JudgedBuilder {
    /// The outcomes a verdict may record, in order.
    pub fn outcomes(&mut self, outcomes: &[&str]) -> &mut Self {
        self.0.outcomes = outcomes.iter().map(ToString::to_string).collect();
        self
    }
    /// The outcomes that pass: a subset of the outcomes.
    pub fn passing(&mut self, passing: &[&str]) -> &mut Self {
        self.0.passing = passing.iter().map(ToString::to_string).collect();
        self
    }
    /// What the rule reads of the entities of `kind`.
    pub fn target(&mut self, kind: &str, f: impl FnOnce(&mut JudgedTargetBuilder)) -> &mut Self {
        let mut b = JudgedTargetBuilder(JudgedTarget {
            kind: kind.to_string(),
            ..JudgedTarget::default()
        });
        f(&mut b);
        self.0.targets.push(b.0);
        self
    }
    /// `kind.field`s set from the verdict's `values` block.
    pub fn values(&mut self, fields: &[&str]) -> &mut Self {
        self.0.values = fields.iter().map(ToString::to_string).collect();
        self
    }
    /// `kind.field`s that hold the verdict's outcome.
    pub fn outcome_fields(&mut self, fields: &[&str]) -> &mut Self {
        self.0.outcome_fields = fields.iter().map(ToString::to_string).collect();
        self
    }
}

/// Builder for [`JudgedTarget`].
pub struct JudgedTargetBuilder(JudgedTarget);

impl JudgedTargetBuilder {
    /// A named input: what `selector` picks from the project.
    pub fn input(&mut self, name: &str, selector: &str) -> &mut Self {
        self.0.inputs.push(JudgedInput {
            name: name.to_string(),
            selector: selector.to_string(),
        });
        self
    }
    /// An entity that writes any of `fields` is not judged.
    pub fn unless_set(&mut self, fields: &[&str]) -> &mut Self {
        self.0.unless_set = fields.iter().map(ToString::to_string).collect();
        self
    }
    /// An entity is judged only once its owner is complete (ADR 0362, the owner state of ADR 0329 D4, read after the
    /// compute passes): while it is pending it owes no verdict. The kind or a scope ancestor declares
    /// `complete_when`.
    pub fn once_complete(&mut self) -> &mut Self {
        self.0.judged_while = Some(OwnerState::Complete.as_str().to_string());
        self
    }
    /// An entity is judged only while its owner is pending (ADR 0362): once it is complete it owes no verdict. The
    /// kind or a scope ancestor declares `complete_when`.
    pub fn while_open(&mut self) -> &mut Self {
        self.0.judged_while = Some(OwnerState::Pending.as_str().to_string());
        self
    }
}
