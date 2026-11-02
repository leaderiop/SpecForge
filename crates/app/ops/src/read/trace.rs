//! `specforge trace` and `specforge.trace`: traceability chains, one
//! operation over the project view (ADR 0015): an entity of a kind an
//! extension declares a route from is traced along it (ADR 0330), any other by
//! its reach both ways; and the vocabulary of what falls short ([`Gap`]: a
//! missing link, or a plan gap).

use serde::Serialize;
use specforge_common::Sym;
use specforge_common::shape::Shape;
use specforge_graph::{Direction, FieldForms, Graph, Node};
use specforge_registry::RegistryBuild;
use std::collections::{BTreeMap, BTreeSet};

mod ask;
mod route;

pub use ask::{
    Ask, ENTITY_WITH_PLAN, PLAN_WITH_ROUTE, ROUTE_OF_ANOTHER_KIND, TraceField, UNKNOWN_ROUTE,
};
pub use route::{Annotated, Partial, RouteMatch, RouteTrace, StepTrace, route_traces};

use specforge_common::{Diagnostic, codes};
use specforge_project::select::Selection;
use specforge_registry::Route;

use crate::read::plan::PlanGap;
use crate::view::{EntityField, ProjectView};
use crate::{Noticed, OpError, OpErrorKind, Refusal};

#[derive(Debug, Serialize, Shape)]
pub struct TraceChain {
    pub entity_id: String,
    pub entity_kind: String,
    pub upstream: Vec<TraceLink>,
    pub downstream: Vec<TraceLink>,
    /// Expected links the traced entity lacks. Only the traced
    /// entity's: each entity's own trace reports its gaps, so a trace of
    /// every entity lists each gap once, and a trace of one entity is not
    /// buried under the gaps of everything below it.
    pub missing: Vec<MissingLink>,
}

impl TraceChain {
    /// The traced entity and every entity its chain links.
    fn reached(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.entity_id.as_str()).chain(
            self.upstream
                .iter()
                .chain(&self.downstream)
                .map(|link| link.entity_id.as_str()),
        )
    }
}

/// Whether a link of a chain exists in the graph (`TraceLinkStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Shape)]
#[serde(rename_all = "lowercase")]
pub enum TraceLinkStatus {
    Resolved,
    Missing,
}

#[derive(Debug, Serialize, Shape)]
#[shape(define)]
pub struct TraceLink {
    pub entity_id: String,
    pub entity_kind: String,
    pub edge_label: String,
    pub depth: usize,
    pub status: TraceLinkStatus,
}

/// An expected link an entity of the chain lacks: a reference field the registries lead its
/// kind to have, which it writes no reference in and no edge of carries (ADR 0232). Always has
/// status `missing`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
pub struct MissingLink {
    /// The entity that lacks the edge.
    pub from: String,
    pub from_kind: String,
    /// The field that would declare the edge — the graph's edge label.
    pub edge_label: String,
    /// The registered edge type the field instantiates, if it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_type: Option<String>,
    /// The kind the edge would point at.
    pub expected_kind: String,
    /// Always 1: the link would leave the traced entity (depth 0).
    pub depth: usize,
    /// The field is required, not just declared.
    pub required: bool,
    pub status: TraceLinkStatus,
}

/// An outgoing edge one kind is expected to have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedEdge {
    /// The field that declares the edge, used as the graph's edge label.
    pub label: String,
    pub edge_type: Option<String>,
    pub target_kind: String,
    pub required: bool,
}

/// The edges each kind is expected to have, derived from the registries
/// only — the core knows nothing about any kind.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceExpectations {
    by_kind: BTreeMap<String, Vec<ExpectedEdge>>,
}

impl TraceExpectations {
    /// No expectations: traces flag nothing as missing.
    pub fn new() -> Self {
        Self::default()
    }

    /// A required reference field is always an expected edge. Otherwise a
    /// reference field is expected when:
    /// - the kind it points at is loaded: a field whose target no loaded
    ///   extension declares is an optional peer;
    /// - the extension that declares the kind declares the field: fields
    ///   another extension adds to the kind (entity enhancements) are
    ///   opt-in;
    /// - it points at another kind: a self-relation (`depends_on`,
    ///   `extends`, `superseded_by`...) is a hierarchy or ordering whose
    ///   roots or leaves lack it by construction.
    pub fn of(build: &RegistryBuild) -> Self {
        let (fields, kinds) = (build.fields(), build.kinds());
        let mut expectations = Self::new();
        for (kind, field, entry) in fields.iter() {
            let Some(target) = entry.declared().target_kind.as_deref() else {
                continue;
            };
            let is_reference = entry.field_type().is_reference();
            let own_field = kinds
                .get(kind)
                .is_some_and(|k| k.source_extension == entry.source_extension());
            if !is_reference
                || !kinds.contains(target)
                || !(own_field || entry.declared().required)
                || (target == kind && !entry.declared().required)
            {
                continue;
            }
            expectations.expect(
                kind,
                ExpectedEdge {
                    label: field.to_string(),
                    edge_type: build
                        .edges()
                        .written_by(kind, field)
                        .map(|edge| edge.label().to_string()),
                    target_kind: target.to_string(),
                    required: entry.declared().required,
                },
            );
        }
        expectations
    }

    /// Expect entities of `kind` to have `edge`.
    pub fn expect(&mut self, kind: &str, edge: ExpectedEdge) {
        let edges = self.by_kind.entry(kind.to_string()).or_default();
        edges.push(edge);
        edges.sort_by(|a, b| a.label.cmp(&b.label));
        edges.dedup_by(|a, b| a.label == b.label);
    }

    /// The edges `kind` is expected to have, ordered by label.
    pub fn for_kind(&self, kind: &str) -> &[ExpectedEdge] {
        self.by_kind.get(kind).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.by_kind.is_empty()
    }
}

/// What a trace request asks to trace (`Ask::Trace`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target<'t> {
    /// One entity's trace.
    Entity(&'t str),
    /// Every entity's trace, ordered by entity id.
    Every,
}

/// What a trace follows from an entity (ADR 0330).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow<'a> {
    /// The route its kind's extensions declare, else its reach both ways.
    Declared,
    /// The route of this qualified name (`<extension>/<route>`, `shop/order-thread`).
    Route(&'a str),
    /// Its reach both ways, whatever routes start at its kind: the generic [`chain`].
    Reach,
}

/// One entity's trace: its reach both ways, or a route's thread.
#[derive(Debug, Serialize, Shape)]
#[serde(untagged)]
pub enum Trace {
    Chain(TraceChain),
    Route(Box<RouteTrace>),
}

/// Some entities' traces, ordered by entity id: the document `specforge
/// trace --format json` writes with no entity, `{schema_version, traces}`.
#[derive(Debug, Serialize)]
pub struct Traces {
    pub schema_version: String,
    pub traces: Vec<Trace>,
}

/// What a trace computed, as the document it serializes as: one entity's
/// trace, `{schema_version, ...}` and the trace's fields, or every entity's,
/// `{schema_version, traces}`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TraceOutcome {
    One(TraceDocument<Trace>),
    Every(Traces),
}

impl TraceOutcome {
    /// Its traces: the traced entity's, or every entity's in id order.
    pub fn traces(&self) -> &[Trace] {
        match self {
            TraceOutcome::One(document) => std::slice::from_ref(&document.trace),
            TraceOutcome::Every(every) => &every.traces,
        }
    }
}

/// One entity's trace as the document `specforge trace <entity> --format
/// json` writes: `{schema_version, entity_id, entity_kind, ...}`.
#[derive(Debug, Serialize, Shape)]
pub struct TraceDocument<T> {
    pub schema_version: String,
    #[serde(flatten)]
    pub trace: T,
}

/// Trace `target` in the view's graph, following `follow`: one entity's
/// trace, or every entity's (with [`Follow::Route`], every entity of the
/// route's start kind). Its notices are what versioning the schema reported
/// (I016, W053; ADR 0111) and each step or annotation the followed routes
/// skip (I210). An entity the graph lacks is `view::not_found` (E003, with
/// its did-you-mean); a route no loaded extension declares, or one of
/// another kind, is invalid input about the route.
pub fn trace(
    view: &ProjectView,
    target: Target,
    follow: Follow,
) -> Result<Noticed<TraceOutcome>, Refusal<TraceField>> {
    if let Target::Entity(entity_id) = target {
        return Ok(trace_one(view, entity_id, follow)?.map(TraceOutcome::One));
    }
    let mut tracer = Tracer::new(view, follow)?;
    let mut nodes: Vec<&Node> = view
        .graph()
        .nodes()
        .into_iter()
        .filter(|node| {
            tracer
                .named
                .is_none_or(|route| route.start == node.kind.raw.as_str())
        })
        .collect();
    nodes.sort_by(|a, b| a.id.raw.as_str().cmp(b.id.raw.as_str()));
    let traces = nodes.into_iter().map(|node| tracer.trace(node)).collect();
    Ok(tracer.noticed(TraceOutcome::Every(Traces {
        schema_version: view.versioned_schema().version().to_string(),
        traces,
    })))
}

/// [`trace`] of [`Target::Entity`]: one entity's trace, as its document.
pub fn trace_one(
    view: &ProjectView,
    entity_id: &str,
    follow: Follow,
) -> Result<Noticed<TraceDocument<Trace>>, Refusal<TraceField>> {
    let mut tracer = Tracer::new(view, follow)?;
    let node = view.entity(entity_id, TraceField::EntityId)?.node;
    if let Some(route) = tracer.named
        && route.start != node.kind.raw.as_str()
    {
        return Err(another_kind(route, entity_id, node.kind.raw.as_str()));
    }
    let trace = tracer.trace(node);
    Ok(tracer.noticed(TraceDocument {
        schema_version: view.versioned_schema().version().to_string(),
        trace,
    }))
}

/// One trace request: what it follows, and the routes it followed, whose skips its notices say.
struct Tracer<'a> {
    view: &'a ProjectView<'a>,
    follow: Follow<'a>,
    /// The route [`Follow::Route`] names.
    named: Option<&'a Route>,
    expectations: TraceExpectations,
    selection: Selection<'a>,
    followed: Vec<&'a Route>,
}

impl<'a> Tracer<'a> {
    /// The request following `follow`; a route no loaded extension declares is refused.
    fn new(view: &'a ProjectView<'a>, follow: Follow<'a>) -> Result<Self, Refusal<TraceField>> {
        let named = match follow {
            Follow::Route(name) => Some(named_route(view, name)?),
            Follow::Declared | Follow::Reach => None,
        };
        Ok(Tracer {
            view,
            follow,
            named,
            expectations: TraceExpectations::of(view.registries()),
            selection: Selection::of(view.graph(), view.registries()),
            followed: Vec::new(),
        })
    }

    /// `node`'s trace: along the route it follows, else its reach.
    fn trace(&mut self, node: &Node) -> Trace {
        let route = match self.follow {
            Follow::Declared => self
                .view
                .registries()
                .routes()
                .followed(node.kind.raw.as_str()),
            Follow::Route(_) => self.named,
            Follow::Reach => None,
        };
        match route {
            Some(route) => {
                if !self.followed.iter().any(|seen| seen.name == route.name) {
                    self.followed.push(route);
                }
                Trace::Route(Box::new(RouteTrace::of(
                    self.view,
                    &self.selection,
                    route,
                    node,
                )))
            }
            None => Trace::Chain(traced(
                self.view.graph(),
                self.view.registries().field_forms(),
                node,
                &self.expectations,
            )),
        }
    }

    /// `value`, with what versioning the schema reported and each followed route's skips.
    fn noticed<T>(self, value: T) -> Noticed<T> {
        let mut notices = self.view.versioned_schema().notices();
        notices.extend(
            self.followed
                .into_iter()
                .flat_map(|route| skipped(self.view, route)),
        );
        Noticed::new(value, notices)
    }
}

/// The route of the qualified `name`: refused, listing the routes, when no loaded extension declares it.
fn named_route<'v>(view: &ProjectView<'v>, name: &str) -> Result<&'v Route, Refusal<TraceField>> {
    let routes = view.registries().routes();
    routes.named(name).ok_or_else(|| {
        let known: Vec<&str> = routes.iter().map(|route| route.name.as_str()).collect();
        let available = if known.is_empty() {
            "no loaded extension declares a route".to_string()
        } else {
            format!("available: {}", known.join(", "))
        };
        OpError::new(
            OpErrorKind::InvalidInput,
            UNKNOWN_ROUTE,
            format!("Unknown route '{name}' ({available})"),
        )
        .about(TraceField::Route)
    })
}

/// The refusal of a route that starts at another kind than the traced entity's.
fn another_kind(route: &Route, entity_id: &str, kind: &str) -> Refusal<TraceField> {
    OpError::new(
        OpErrorKind::InvalidInput,
        ROUTE_OF_ANOTHER_KIND,
        format!(
            "route '{}' starts at kind '{}', and '{entity_id}' is a '{kind}'",
            route.name, route.start
        ),
    )
    .about(TraceField::Route)
}

/// I210 for each step or annotation `route` skips: it reaches a kind of an optional peer the project does not load.
fn skipped(view: &ProjectView, route: &Route) -> Vec<Diagnostic> {
    let candidates = view.registries().candidates();
    route
        .skipped
        .iter()
        .map(|skip| {
            Diagnostic::new(
                codes::I210,
                format!(
                    "route '{}' skips {skip}: kind '{}' is not loaded",
                    route.name, skip.kind
                ),
            )
            .with_suggestion(candidates.kind_remedy(&skip.kind))
        })
        .collect()
}

/// The chain of `entity_id` both ways, with every edge the registries lead
/// it to have, and it lacks, flagged as missing: the reach, whatever routes
/// start at its kind. Its notices are what versioning the schema reported
/// (I016, W053; ADR 0111). An entity the graph lacks is `view::not_found`
/// (E003, with its did-you-mean).
pub fn chain(
    view: &ProjectView,
    entity_id: &str,
) -> Result<Noticed<TraceDocument<TraceChain>>, Refusal<EntityField>> {
    let expectations = TraceExpectations::of(view.registries());
    let graph = view.graph();
    let forms = view.registries().field_forms();
    let versioned = view.versioned_schema();
    let node = view.entity(entity_id, EntityField::EntityId)?.node;
    let chain = traced(graph, forms, node, &expectations);
    Ok(Noticed::new(
        TraceDocument {
            schema_version: versioned.version().to_string(),
            trace: chain,
        },
        versioned.notices(),
    ))
}

/// What the trace chains of some entities reach, and which of it is
/// unverified: the trace prompt's read view (ADR 0015 D4, P1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Affected {
    /// The seeds and every entity their chains reach, sorted by ID.
    pub entities: Vec<String>,
    /// The affected entities that count toward coverage and are not
    /// proven (`ProjectCoverage::is_unverified`), sorted by ID.
    pub unverified: Vec<String>,
}

/// The entities affected by `seeds` over `view`: each seed's chain as
/// `trace(view, Target::Entity(seed), Follow::Reach)` computes it. A seed the graph lacks
/// is `view::not_found` (E003, with its
/// did-you-mean); a recorded report that cannot be used is the coverage
/// view's (E045).
pub fn affected<'s>(
    view: &ProjectView,
    seeds: impl IntoIterator<Item = &'s str>,
) -> Result<Affected, OpError> {
    let expectations = TraceExpectations::of(view.registries());
    let graph = view.graph();
    let forms = view.registries().field_forms();
    let mut entities = BTreeSet::new();
    for seed in seeds {
        let node = view
            .entity(seed, EntityField::EntityId)
            .map_err(|refusal| refusal.error)?
            .node;
        let chain = traced(graph, forms, node, &expectations);
        entities.extend(chain.reached().map(str::to_string));
    }
    let coverage = view.coverage()?;
    let unverified = entities
        .iter()
        .filter(|id| coverage.is_unverified(id))
        .cloned()
        .collect();
    Ok(Affected {
        entities: entities.into_iter().collect(),
        unverified,
    })
}

/// The chain of `entity_id` both ways, with every expected link it lacks
/// flagged as missing.
fn traced(
    graph: &Graph,
    forms: &FieldForms,
    root: &Node,
    expectations: &TraceExpectations,
) -> TraceChain {
    let entity_id = root.id.raw.as_str();
    TraceChain {
        entity_id: entity_id.to_string(),
        entity_kind: root.kind.raw.to_string(),
        upstream: links(graph, entity_id, Direction::Incoming),
        downstream: links(graph, entity_id, Direction::Outgoing),
        missing: missing_links(graph, forms, root, expectations),
    }
}

/// What falls short, in one vocabulary (CONTEXT.md), as the `McpTraceGap` row
/// the trace prompt lists: a missing link, an expected edge a traced entity
/// lacks (`source_entity` the entity, `target_entity` the kind it would point
/// at, `missing_link_type` "missing_link"); or a plan gap, how an agent plan
/// falls short of the graph (its own fields). A written reference to an entity that does not
/// exist is neither: it is E003, `check`'s business (ADR 0232).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct Gap {
    source_entity: String,
    target_entity: String,
    missing_link_type: String,
    gap_context: String,
}

impl From<&MissingLink> for Gap {
    fn from(link: &MissingLink) -> Self {
        let required = if link.required { " (required)" } else { "" };
        Gap {
            source_entity: link.from.clone(),
            target_entity: link.expected_kind.clone(),
            missing_link_type: "missing_link".to_string(),
            gap_context: format!(
                "{} '{}' has no '{}' link to {} {}{required}",
                link.from_kind,
                link.from,
                link.edge_label,
                article(&link.expected_kind),
                link.expected_kind
            ),
        }
    }
}

/// "an" before a kind name that starts with a vowel, else "a".
fn article(kind: &str) -> &'static str {
    if kind.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

impl From<&PlanGap> for Gap {
    fn from(gap: &PlanGap) -> Self {
        Gap {
            source_entity: gap.source.clone(),
            target_entity: gap.target.clone(),
            missing_link_type: gap.kind.as_str().to_string(),
            gap_context: gap.context.clone(),
        }
    }
}

/// The expected links `node` lacks, one hop from it: each expected field
/// it does not give its link ([`Graph::linked_fields`], ADR 0232). A
/// reference that does not resolve is written, so it is E003, `check`'s,
/// never a missing link.
fn missing_links(
    graph: &Graph,
    forms: &FieldForms,
    node: &Node,
    expectations: &TraceExpectations,
) -> Vec<MissingLink> {
    let (entity_id, kind) = (node.id.raw.as_str(), node.kind.raw.as_str());
    let linked = graph.linked_fields(forms, node);
    expectations
        .for_kind(kind)
        .iter()
        .filter(|expected| !linked.contains(&Sym::new(&expected.label)))
        .map(|expected| MissingLink {
            from: entity_id.to_string(),
            from_kind: kind.to_string(),
            edge_label: expected.label.clone(),
            edge_type: expected.edge_type.clone(),
            expected_kind: expected.target_kind.clone(),
            depth: 1,
            required: expected.required,
            status: TraceLinkStatus::Missing,
        })
        .collect()
}

/// `entity_id`'s links one way: what the graph's directed reach reaches, by depth, then id.
fn links(graph: &Graph, entity_id: &str, direction: Direction) -> Vec<TraceLink> {
    let mut links: Vec<TraceLink> = graph
        .reach_directed(entity_id, direction)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|reached| {
            let (_, label) = reached.via?; // the root has none
            let node = graph.node(reached.id.as_str())?;
            Some(TraceLink {
                entity_id: reached.id.to_string(),
                entity_kind: node.kind.raw.to_string(),
                edge_label: label.to_string(),
                depth: reached.depth,
                status: TraceLinkStatus::Resolved,
            })
        })
        .collect();
    links.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.entity_id.cmp(&b.entity_id)));
    links
}

#[cfg(test)]
mod tests {
    use super::article;

    #[test]
    fn the_missing_link_sentence_writes_an_before_a_vowel() {
        assert_eq!(article("event"), "an");
        assert_eq!(article("invariant"), "an");
        assert_eq!(article("port"), "a");
    }
}
