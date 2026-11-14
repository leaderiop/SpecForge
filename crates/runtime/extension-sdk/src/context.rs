//! Declaring the context routes a role reads (protocol 2.1.0, ADR 0356). The builders don't validate: the host's
//! registry does, where every loaded kind, agent and setting is known (E123, E124).

use specforge_protocol_types::{
    ContextDescriptor, ContextRecordDescriptor, ContextSectionDescriptor, ContextTier,
};

use crate::ContributionsBuilder;

impl ContributionsBuilder {
    /// Declare a context route: what a role reads of one entity of its subject kind, kebab `name`, unique among this
    /// extension's context routes.
    ///
    /// ```
    /// # use specforge_extension_sdk::prelude::*;
    /// # let mut c = ContributionsBuilder::default();
    /// c.context("executor", |r| {
    ///     r.subject("plan")
    ///         .agent("gsd-executor")
    ///         .budget(6000)
    ///         .core("plan", "self")
    ///         .enrich("decisions", "self.phase<decision.phase")
    ///         .settings(&["plan_check"]);
    /// });
    /// ```
    pub fn context(&mut self, name: &str, f: impl FnOnce(&mut ContextBuilder)) -> &mut Self {
        let mut b = ContextBuilder(ContextDescriptor {
            name: name.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.decl.contexts.push(b.0);
        self
    }
}

/// Builder for a [`ContextDescriptor`].
pub struct ContextBuilder(ContextDescriptor);

impl ContextBuilder {
    /// The kind of the entity a pack is read for.
    pub fn subject(&mut self, kind: &str) -> &mut Self {
        self.0.subject = kind.to_string();
        self
    }
    pub fn description(&mut self, text: &str) -> &mut Self {
        self.0.description = Some(text.to_string());
        self
    }
    /// The extension's own agent that reads it: its body gets a Context section.
    pub fn agent(&mut self, agent: &str) -> &mut Self {
        self.0.agent = Some(agent.to_string());
        self
    }
    /// The token budget a read takes when it names none.
    pub fn budget(&mut self, tokens: u32) -> &mut Self {
        self.0.budget = tokens;
        self
    }
    /// A section a budget never cuts: the entities, files or tests `selector` picks from the subject.
    pub fn core(&mut self, name: &str, selector: &str) -> &mut Self {
        self.section(name, selector, ContextTier::Core)
    }
    /// A section served whole while it fits, after the sections declared before it.
    pub fn enrich(&mut self, name: &str, selector: &str) -> &mut Self {
        self.section(name, selector, ContextTier::Enrich)
    }
    /// Record this route's pack in `field` (`kind.field`, a `context` field of this extension), its subject found from
    /// the entity holding it by the chain `subject` (`self`, `self.phase`). Name the code a moved one raises with
    /// [`Self::stale`].
    pub fn record(&mut self, field: &str, subject: &str) -> &mut Self {
        self.push_record(field, subject, None)
    }
    /// [`Self::record`], a stale record's finding suggesting `suggestion` (`{entity}`, `{subject}` and `{route}` are
    /// filled in).
    pub fn record_suggesting(&mut self, field: &str, subject: &str, suggestion: &str) -> &mut Self {
        self.push_record(field, subject, Some(suggestion))
    }
    /// The code of this extension's catalogue a record whose pack moved raises, local (`W900`).
    pub fn stale(&mut self, code: &str) -> &mut Self {
        self.0.stale = Some(code.to_string());
        self
    }
    fn push_record(&mut self, field: &str, subject: &str, suggestion: Option<&str>) -> &mut Self {
        self.0.records.push(ContextRecordDescriptor {
            field: field.to_string(),
            subject: subject.to_string(),
            suggestion: suggestion.map(str::to_string),
        });
        self
    }
    /// Settings of this extension whose values the pack carries.
    pub fn settings(&mut self, names: &[&str]) -> &mut Self {
        self.0
            .settings
            .extend(names.iter().map(|name| name.to_string()));
        self
    }
    fn section(&mut self, name: &str, selector: &str, tier: ContextTier) -> &mut Self {
        self.0.sections.push(ContextSectionDescriptor {
            name: name.to_string(),
            selector: selector.to_string(),
            tier,
        });
        self
    }
}
