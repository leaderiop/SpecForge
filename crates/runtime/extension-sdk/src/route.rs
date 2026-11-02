//! Declaring the routes `specforge trace` follows (protocol 1.15.0, ADR 0330). The builders don't validate: the
//! host's registry does, where every loaded kind is known (E118).

use specforge_protocol_types::{
    ROUTE_START, RouteAnnotationDescriptor, RouteDescriptor, RouteStepDescriptor,
};

use crate::ContributionsBuilder;

impl ContributionsBuilder {
    /// Declare a route `specforge trace` follows from the entities of its start kind: kebab `name`, unique among
    /// this extension's routes.
    ///
    /// ```
    /// # use specforge_extension_sdk::prelude::*;
    /// # let mut c = ContributionsBuilder::default();
    /// c.route("requirement-thread", |r| {
    ///     r.start("requirement")
    ///         .step("plan", "<plan.requirements", |s| { s.label("plan"); })
    ///         .step("task", "<task.plan", |_| {});
    /// });
    /// ```
    pub fn route(&mut self, name: &str, f: impl FnOnce(&mut RouteBuilder)) -> &mut Self {
        let mut b = RouteBuilder(RouteDescriptor {
            name: name.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.decl.routes.push(b.0);
        self
    }
}

/// Builder for a [`RouteDescriptor`].
pub struct RouteBuilder(RouteDescriptor);

impl RouteBuilder {
    /// The kind whose entities the route starts from.
    pub fn start(&mut self, kind: &str) -> &mut Self {
        self.0.start = kind.to_string();
        self
    }
    pub fn description(&mut self, text: &str) -> &mut Self {
        self.0.description = Some(text.to_string());
        self
    }
    /// The route `trace` follows when several start at its kind (W195 otherwise).
    pub fn default_route(&mut self) -> &mut Self {
        self.0.default = true;
        self
    }
    /// A step: the entities `via` (selector steps, `<plan.requirements`) selects from each entity of the previous
    /// step (the start for the first), unless [`RouteStepBuilder::from_start`] or [`RouteStepBuilder::from_step`] says
    /// otherwise.
    pub fn step(
        &mut self,
        name: &str,
        via: &str,
        f: impl FnOnce(&mut RouteStepBuilder),
    ) -> &mut Self {
        let mut b = RouteStepBuilder(RouteStepDescriptor {
            name: name.to_string(),
            via: via.to_string(),
            ..Default::default()
        });
        f(&mut b);
        self.0.steps.push(b.0);
        self
    }
}

/// Builder for a [`RouteStepDescriptor`].
pub struct RouteStepBuilder(RouteStepDescriptor);

impl RouteStepBuilder {
    /// Go on from the start entity.
    pub fn from_start(&mut self) -> &mut Self {
        self.0.from = Some(ROUTE_START.to_string());
        self
    }
    /// Go on from each match of the earlier step `step`.
    pub fn from_step(&mut self, step: &str) -> &mut Self {
        self.0.from = Some(step.to_string());
        self
    }
    /// The word printed before each match's name (`plan` → `plan 01-01`).
    pub fn label(&mut self, label: &str) -> &mut Self {
        self.0.label = Some(label.to_string());
        self
    }
}

/// What the thread prints of an entity: the start's ([`RouteBuilder`]) or each match of a step's
/// ([`RouteStepBuilder`]).
pub trait Annotates {
    /// The annotations declared so far.
    fn annotations(&mut self) -> &mut Vec<RouteAnnotationDescriptor>;

    /// Print the entity's `field`.
    fn field(&mut self, field: &str) -> &mut Self {
        self.annotate(|a| {
            a.field(field);
        })
    }
    /// Print the entity's judgement by the judged rule `code`.
    fn judgement(&mut self, code: &str) -> &mut Self {
        self.annotate(|a| {
            a.judgement(code);
        })
    }
    /// Print an annotation of the entity.
    fn annotate(&mut self, f: impl FnOnce(&mut RouteAnnotationBuilder)) -> &mut Self {
        self.annotations().push(RouteAnnotationBuilder::built(f));
        self
    }
}

impl Annotates for RouteBuilder {
    fn annotations(&mut self) -> &mut Vec<RouteAnnotationDescriptor> {
        &mut self.0.annotations
    }
}

impl Annotates for RouteStepBuilder {
    fn annotations(&mut self) -> &mut Vec<RouteAnnotationDescriptor> {
        &mut self.0.annotations
    }
}

/// Builder for a [`RouteAnnotationDescriptor`]: exactly one of [`field`](Self::field) and
/// [`judgement`](Self::judgement).
pub struct RouteAnnotationBuilder(RouteAnnotationDescriptor);

impl RouteAnnotationBuilder {
    fn built(f: impl FnOnce(&mut RouteAnnotationBuilder)) -> RouteAnnotationDescriptor {
        let mut b = RouteAnnotationBuilder(RouteAnnotationDescriptor::default());
        f(&mut b);
        b.0
    }
    /// A field (or `block.member`) of the entity, or of the entities [`of`](Self::of) picks.
    pub fn field(&mut self, field: &str) -> &mut Self {
        self.0.field = Some(field.to_string());
        self
    }
    /// Selector steps from the entity to the entities whose field is read (`<test_run.test`).
    pub fn of(&mut self, via: &str) -> &mut Self {
        self.0.of = Some(via.to_string());
        self
    }
    /// A judged rule's code: the extension's own (`E501`) or qualified (`gsd/E501`).
    pub fn judgement(&mut self, code: &str) -> &mut Self {
        self.0.judgement = Some(code.to_string());
        self
    }
    /// What prints when there is nothing to print (`not run`).
    pub fn absent(&mut self, text: &str) -> &mut Self {
        self.0.absent = Some(text.to_string());
        self
    }
}
