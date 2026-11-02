//! The routes `specforge trace` follows (ADR 0330), each checked once against the loaded kinds, fields and judged
//! rules: a route that cannot be followed is E118 and is not registered; several from one kind without one default
//! are W195. A part that reaches a kind of an optional peer the project does not load is skipped, not reported.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_diagnostics::qualify;
use specforge_protocol_types::{
    ExtensionDeclaration, ROUTE_START, RouteAnnotationDescriptor, RouteDescriptor, is_valid_short,
};

use crate::compilation::has_unloaded_optional_peer;
use crate::judged::{Chain, Head, Reader, Selector, WalkError, is_word, walk};
use crate::rules::{Registries, Rules};

/// Where a step goes on from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parent {
    /// The start entity.
    Start,
    /// Each match of the registered step at this index of [`Route::steps`].
    Step(usize),
}

/// What an annotation prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAnnotation {
    /// The values of `field` on the entity, or on each entity `of` selects from it.
    Field {
        of: Option<Chain>,
        field: String,
        absent: Option<String>,
    },
    /// The entity's judgement by the judged rule of the qualified `code`.
    Judgement {
        code: String,
        absent: Option<String>,
    },
}

impl RouteAnnotation {
    /// What prints when there is nothing to print.
    pub fn absent(&self) -> Option<&str> {
        match self {
            RouteAnnotation::Field { absent, .. } | RouteAnnotation::Judgement { absent, .. } => {
                absent.as_deref()
            }
        }
    }

    /// The field or the qualified code it reads.
    pub fn name(&self) -> &str {
        match self {
            RouteAnnotation::Field { field, .. } => field,
            RouteAnnotation::Judgement { code, .. } => code,
        }
    }
}

/// One registered step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteStep {
    pub name: String,
    pub from: Parent,
    /// The selector steps from each entity of `from`, as a chain from `self`.
    pub via: Chain,
    pub label: Option<String>,
    pub annotations: Vec<RouteAnnotation>,
}

impl RouteStep {
    /// What the thread calls the step: its label, else its name.
    pub fn title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }
}

/// A step or an annotation dropped because it reaches `kind`, a kind of an optional peer the project does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    /// The step skipped, or whose annotation is: its name, or [`ROUTE_START`] for the start's annotations.
    pub step: String,
    /// The annotation skipped (its field or code), when the step itself is not.
    pub annotation: Option<String>,
    pub kind: String,
}

impl std::fmt::Display for Skipped {
    /// `step 'plan'`, or `annotation 'status' of step 'test'` (`of the start` for the start's).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let step = match self.step.as_str() {
            ROUTE_START => "the start".to_string(),
            name => format!("step '{name}'"),
        };
        match &self.annotation {
            None => f.write_str(&step),
            Some(annotation) => write!(f, "annotation '{annotation}' of {step}"),
        }
    }
}

/// A registered route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Qualified by its extension's short name: `shop/order-thread`.
    pub name: String,
    pub extension: String,
    pub description: Option<String>,
    pub start: String,
    pub default: bool,
    pub annotations: Vec<RouteAnnotation>,
    /// The registered steps, in declared order; each goes on only from an earlier one.
    pub steps: Vec<RouteStep>,
    pub skipped: Vec<Skipped>,
}

/// Every registered route, and the one followed from each start kind.
#[derive(Debug, Clone, Default)]
pub struct RouteRegistry {
    routes: Vec<Route>,
    followed: BTreeMap<String, usize>,
}

impl RouteRegistry {
    /// Every route, in load order.
    pub fn iter(&self) -> impl Iterator<Item = &Route> {
        self.routes.iter()
    }

    /// The route `trace` follows from an entity of `kind`, when any starts there.
    pub fn followed(&self, kind: &str) -> Option<&Route> {
        self.followed.get(kind).map(|&at| &self.routes[at])
    }

    /// The route of the qualified `name`.
    pub fn named(&self, name: &str) -> Option<&Route> {
        self.routes.iter().find(|route| route.name == name)
    }

    /// The routes that start at `kind`, in load order.
    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Route> + 'a {
        self.routes.iter().filter(move |route| route.start == kind)
    }
}

/// The routes `declarations` (in load order) declare, checked against `registries` and the judged rules of `rules`;
/// each route that cannot be followed is E118, each kind with several routes and not one default W195.
pub(crate) fn build(
    declarations: &[ExtensionDeclaration],
    registries: Registries<'_>,
    rules: &Rules,
) -> (RouteRegistry, Vec<Diagnostic>) {
    let loaded: Vec<&str> = declarations.iter().map(|d| d.name()).collect();
    let mut registry = RouteRegistry::default();
    let mut diagnostics = Vec::new();
    for declaration in declarations {
        let mut names = BTreeSet::new();
        for descriptor in &declaration.routes {
            let check = Check {
                declaration,
                registries,
                rules,
                optional: has_unloaded_optional_peer(declaration, &loaded),
            };
            let checked = if names.insert(descriptor.name.as_str()) {
                check.route(descriptor)
            } else {
                Err("a route's name is unique in its extension".to_string())
            };
            match checked {
                Ok(Some(route)) => registry.routes.push(route),
                Ok(None) => {}
                Err(why) => diagnostics.push(
                    Diagnostic::new(
                        codes::E118,
                        format!(
                            "extension '{}': route '{}': {why} — the route was not registered",
                            declaration.name(),
                            descriptor.name
                        ),
                    )
                    .with_suggestion(format!(
                        "fix the route's declaration in {}",
                        declaration.name()
                    )),
                ),
            }
        }
    }
    diagnostics.extend(registry.follow());
    (registry, diagnostics)
}

impl RouteRegistry {
    /// Pick the route followed from each start kind: the only one, the one marked default, else the first in load
    /// order with W195.
    fn follow(&mut self) -> Vec<Diagnostic> {
        let mut by_kind: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (at, route) in self.routes.iter().enumerate() {
            by_kind.entry(&route.start).or_default().push(at);
        }
        let mut diagnostics = Vec::new();
        let mut followed = BTreeMap::new();
        for (kind, ats) in by_kind {
            let defaults: Vec<usize> = ats
                .iter()
                .copied()
                .filter(|&at| self.routes[at].default)
                .collect();
            let chosen = match (&ats[..], &defaults[..]) {
                ([one], _) | (_, [one]) => *one,
                _ => {
                    let names: Vec<&str> = ats
                        .iter()
                        .map(|&at| self.routes[at].name.as_str())
                        .collect();
                    let marked = if defaults.is_empty() {
                        "none is"
                    } else {
                        "several are"
                    };
                    diagnostics.push(
                        Diagnostic::new(
                            codes::W195,
                            format!(
                                "kind '{kind}' starts {} trace routes ({}) and {marked} marked default: '{}' is \
                                 followed",
                                ats.len(),
                                names.join(", "),
                                names[0]
                            ),
                        )
                        .with_suggestion(format!(
                            "mark one route `default`, or name the route to follow: {}",
                            specforge_common::remedy::command(
                                "specforge trace <entity> --route {}",
                                &[names[0]]
                            )
                        )),
                    );
                    ats[0]
                }
            };
            followed.insert(kind.to_string(), chosen);
        }
        self.followed = followed;
        diagnostics
    }
}

/// What one route is checked against.
struct Check<'a> {
    declaration: &'a ExtensionDeclaration,
    registries: Registries<'a>,
    rules: &'a Rules,
    /// Whether its extension declares an optional peer the project does not load: then a part that reaches an
    /// unloaded kind is skipped.
    optional: bool,
}

/// What a declared step reached.
enum Reached {
    /// It is registered at `index` of the route's steps, and its matches are of `kinds`.
    Step {
        index: usize,
        kinds: BTreeSet<String>,
    },
    /// It was skipped for `kind`, an optional peer's unloaded kind.
    Skipped { kind: String },
}

impl Check<'_> {
    /// The route registered; `None` when its start kind is an unloaded optional peer's; `Err` the reason it can't be
    /// followed (E118).
    fn route(&self, d: &RouteDescriptor) -> Result<Option<Route>, String> {
        if !is_valid_short(&d.name) {
            return Err("a route's name is kebab case".into());
        }
        if d.steps.is_empty() {
            return Err("a route has at least one step".into());
        }
        if !self.registries.kinds.contains(&d.start) {
            return match self.optional {
                true => Ok(None),
                false => Err(format!("start kind '{}' is not loaded", d.start)),
            };
        }
        let start = BTreeSet::from([d.start.clone()]);
        let mut skipped = Vec::new();
        let annotations = self.annotations(ROUTE_START, &d.annotations, &start, &mut skipped)?;
        let mut steps: Vec<RouteStep> = Vec::new();
        let mut names: Vec<&str> = Vec::new();
        let mut reached: Vec<Reached> = Vec::new();
        for (at, step) in d.steps.iter().enumerate() {
            let name = step.name.as_str();
            if !is_word(name) || name == ROUTE_START || names.contains(&name) {
                return Err(format!(
                    "step '{name}': a step's name is a word, unique in the route, and not '{ROUTE_START}'"
                ));
            }
            let parent = match step.from.as_deref() {
                None => at.checked_sub(1),
                Some(ROUTE_START) => None,
                Some(from) => Some(names.iter().position(|n| *n == from).ok_or_else(|| {
                    format!("step '{name}': from '{from}' names no earlier step")
                })?),
            };
            names.push(name);
            let via = selector_steps(&step.via)
                .map_err(|why| format!("step '{name}': via '{}': {why}", step.via))?;
            let (from, kinds) = match parent.map(|p| &reached[p]) {
                None => (Parent::Start, &start),
                Some(Reached::Step { index, kinds }) => (Parent::Step(*index), kinds),
                // It goes on from a skipped step: skipped for the same kind.
                Some(Reached::Skipped { kind }) => {
                    let kind = kind.clone();
                    skip_step(&mut skipped, &mut reached, name, kind);
                    continue;
                }
            };
            let matched = match self.walk(&via, kinds) {
                Ok(matched) => matched,
                Err(WalkError::Unloaded { kind, .. }) => {
                    skip_step(&mut skipped, &mut reached, name, kind);
                    continue;
                }
                Err(e) => return Err(format!("step '{name}': via '{}': {}", step.via, e.why())),
            };
            let annotations = self.annotations(name, &step.annotations, &matched, &mut skipped)?;
            steps.push(RouteStep {
                name: name.to_string(),
                from,
                via,
                label: step.label.clone(),
                annotations,
            });
            reached.push(Reached::Step {
                index: steps.len() - 1,
                kinds: matched,
            });
        }
        Ok(Some(Route {
            name: format!("{}/{}", self.declaration.short(), d.name),
            extension: self.declaration.name().to_string(),
            description: d.description.clone(),
            start: d.start.clone(),
            default: d.default,
            annotations,
            steps,
            skipped,
        }))
    }

    /// The kinds `chain` reaches from `kinds`; [`WalkError::Unloaded`] only when it reaches an optional peer's
    /// unloaded kind (the part is skipped), any other failure [`WalkError::Invalid`] (E118).
    fn walk(&self, chain: &Chain, kinds: &BTreeSet<String>) -> Result<BTreeSet<String>, WalkError> {
        walk(
            chain,
            chain.steps.len(),
            kinds,
            self.registries,
            Reader::Route,
        )
        .map_err(|e| match e {
            WalkError::Unloaded { .. } if self.optional => e,
            other => WalkError::Invalid(other.why()),
        })
    }

    /// The annotations of `step` (a step's name, or [`ROUTE_START`]) over entities of `kinds`, each checked; one
    /// skipped for an optional peer is recorded in `skipped`.
    fn annotations(
        &self,
        step: &str,
        declared: &[RouteAnnotationDescriptor],
        kinds: &BTreeSet<String>,
        skipped: &mut Vec<Skipped>,
    ) -> Result<Vec<RouteAnnotation>, String> {
        let mut annotations = Vec::new();
        for a in declared {
            let named = a.field.as_deref().or(a.judgement.as_deref()).unwrap_or("");
            let at = |why: String| format!("{step} annotation '{named}': {why}");
            let absent = a.absent.clone();
            let annotation = match (&a.field, &a.judgement, &a.of) {
                (Some(field), None, of) => {
                    let (of, kinds) = match of {
                        None => (None, kinds.clone()),
                        Some(text) => {
                            let chain = selector_steps(text)
                                .map_err(|why| at(format!("of '{text}': {why}")))?;
                            match self.walk(&chain, kinds) {
                                Ok(reached) => (Some(chain), reached),
                                Err(WalkError::Unloaded { kind, .. }) => {
                                    skipped.push(Skipped {
                                        step: step.to_string(),
                                        annotation: Some(named.to_string()),
                                        kind,
                                    });
                                    continue;
                                }
                                Err(e) => return Err(at(format!("of '{text}': {}", e.why()))),
                            }
                        }
                    };
                    if let Some(kind) = kinds
                        .iter()
                        .find(|k| self.registries.fields.get_path(k, field).is_none())
                    {
                        return Err(at(format!("kind '{kind}' declares no field '{field}'")));
                    }
                    RouteAnnotation::Field {
                        of,
                        field: field.clone(),
                        absent,
                    }
                }
                (None, Some(code), None) => {
                    let code = match code.contains('/') {
                        true => code.clone(),
                        false => qualify(self.declaration.name(), code),
                    };
                    let rule = self
                        .rules
                        .judged()
                        .find(|rule| rule.code == code)
                        .ok_or_else(|| at(format!("'{code}' names no judged rule")))?;
                    if let Some(kind) = kinds.iter().find(|k| rule.target(k).is_none()) {
                        return Err(at(format!(
                            "judged rule '{code}' does not judge kind '{kind}'"
                        )));
                    }
                    RouteAnnotation::Judgement { code, absent }
                }
                (None, Some(_), Some(_)) => {
                    return Err(at("`of` comes only with a field".into()));
                }
                _ => {
                    return Err(at(
                        "an annotation has exactly one of field and judgement".into()
                    ));
                }
            };
            annotations.push(annotation);
        }
        Ok(annotations)
    }
}

/// Record `step` skipped for `kind`, an optional peer's unloaded kind; a step that goes on from it is skipped too.
fn skip_step(skipped: &mut Vec<Skipped>, reached: &mut Vec<Reached>, step: &str, kind: String) {
    skipped.push(Skipped {
        step: step.to_string(),
        annotation: None,
        kind: kind.clone(),
    });
    reached.push(Reached::Skipped { kind });
}

/// `via` (selector steps with no head: `<invoice.order`, `.items`) read as a chain from `self` with at least one
/// step.
fn selector_steps(via: &str) -> Result<Chain, String> {
    match Selector::parse(&format!("self{via}"))? {
        Selector::Entities(chain) if chain.head == Head::This && !chain.steps.is_empty() => {
            Ok(chain)
        }
        _ => Err("selector steps start with '.' or '<' (`<invoice.order`, `.items`)".into()),
    }
}
