//! A trace along a route an extension declares (ADR 0330): from one entity of the route's start kind, each step's
//! matches under the match they came from, what each annotation prints of them, and the thread as text.

use serde::Serialize;
use serde_json::Value;
use specforge_common::shape::Shape;
use specforge_graph::{Node, Reading, wire};
use specforge_project::select::Selection;
use specforge_registry::{Parent, Route, RouteAnnotation};

use crate::view::ProjectView;

/// One entity traced along a route: its matches, step by step, and its thread as `--partial` prints it.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct RouteTrace {
    pub entity_id: String,
    pub entity_kind: String,
    /// The route's qualified name (`<extension>/<route>`, `shop/order-thread`).
    pub route: String,
    /// What the start entity's annotations print.
    pub annotations: Vec<Annotated>,
    /// Every step the route registered, in declared order, with every match (partial branches included).
    pub steps: Vec<StepTrace>,
    /// The steps and annotations the route skips, because they reach a kind no loaded extension declares, as I210
    /// names them: `step 'test'`, `annotation 'status' of step 'test'`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    /// At least one path reaches every step: the default output prints the thread.
    pub complete: bool,
    /// The steps the thread falls short at, in declared order: each is reached (it goes on from the start or from a
    /// step with a match) and has no kept match, though every reached step that goes on from it has one.
    pub missing: Vec<String>,
    /// The thread over every match, `no <step>` markers included: what `--partial` prints.
    pub text: String,
    /// The start entity's name, and its name and annotations as the thread prints them.
    #[serde(skip)]
    start: Segment,
}

/// One step of a route trace.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct StepTrace {
    pub name: String,
    /// `start`, or the name of the step it goes on from.
    pub from: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub matches: Vec<RouteMatch>,
    /// The index of the step it goes on from, when not the start.
    #[serde(skip)]
    parent: Option<usize>,
    /// What the thread calls the step ([`specforge_registry::RouteStep::title`]).
    #[serde(skip)]
    title: String,
}

impl StepTrace {
    fn title(&self) -> &str {
        &self.title
    }
}

/// An entity a step reached from one match of the step it goes on from.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct RouteMatch {
    pub entity_id: String,
    pub entity_kind: String,
    pub name: String,
    /// The entity it was reached from.
    pub parent: String,
    pub annotations: Vec<Annotated>,
    /// The index of the match it was reached from, in its parent step's matches; none for the start.
    #[serde(skip)]
    parent_at: Option<usize>,
    #[serde(skip)]
    segment: Segment,
}

/// What one annotation printed of one entity, by what it reads (the registry's [`RouteAnnotation`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Shape)]
#[serde(tag = "source", rename_all = "lowercase")]
#[shape(define)]
pub enum Annotated {
    /// A field's values.
    Field {
        /// The field (or `block.member`).
        name: String,
        /// Its values, in order, each once.
        values: Vec<String>,
        /// What the thread prints for it; none when it prints nothing.
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// A judged rule's judgement.
    Judgement {
        /// The judged rule's qualified code.
        code: String,
        /// The judgement's state (`fresh`, `stale`, `missing`, ...); none when the rule has not judged it.
        #[serde(skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        /// The judgement's recorded outcome.
        #[serde(skip_serializing_if = "Option::is_none")]
        outcome: Option<String>,
        /// What the thread prints for it; none when it prints nothing.
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

impl Annotated {
    /// What the thread prints for it; none when it prints nothing.
    pub fn text(&self) -> Option<&str> {
        match self {
            Annotated::Field { text, .. } | Annotated::Judgement { text, .. } => text.as_deref(),
        }
    }
}

/// An entity as the thread prints it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Segment {
    name: String,
    text: String,
}

/// Whether a thread is printed when it is incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partial {
    /// Only a complete thread, over its kept matches.
    No,
    /// The thread over every match, with `no <step>` markers.
    Yes,
}

impl RouteTrace {
    /// `entity` (an entity of the route's start kind) traced along `route`, its steps evaluated by `selection`.
    pub(super) fn of(
        view: &ProjectView,
        selection: &Selection,
        route: &Route,
        entity: &Node,
    ) -> RouteTrace {
        let graph = view.graph();
        let annotations = annotate(view, selection, &route.annotations, entity);
        let start = Segment::of(None, entity, &annotations);
        let mut steps: Vec<StepTrace> = Vec::with_capacity(route.steps.len());
        for step in &route.steps {
            let parent = match step.from {
                Parent::Start => None,
                Parent::Step(at) => Some(at),
            };
            // Each match of the step it goes on from (the start's one, with no index), in order.
            let froms: Vec<(Option<usize>, &str)> = match parent {
                None => vec![(None, entity.id.raw.as_str())],
                Some(at) => steps[at]
                    .matches
                    .iter()
                    .enumerate()
                    .map(|(index, m)| (Some(index), m.entity_id.as_str()))
                    .collect(),
            };
            let mut matches = Vec::new();
            for (parent_at, from) in froms {
                for id in selection.entities(from, &step.via) {
                    let Some(node) = graph.node(&id) else {
                        continue;
                    };
                    let annotations = annotate(view, selection, &step.annotations, node);
                    matches.push(RouteMatch {
                        entity_id: id.clone(),
                        entity_kind: node.kind.raw.to_string(),
                        name: node.name().to_string(),
                        parent: from.to_string(),
                        segment: Segment::of(step.label.as_deref(), node, &annotations),
                        annotations,
                        parent_at,
                    });
                }
            }
            steps.push(StepTrace {
                name: step.name.clone(),
                from: parent.map_or_else(
                    || specforge_protocol_types::ROUTE_START.to_string(),
                    |at| route.steps[at].name.clone(),
                ),
                label: step.label.clone(),
                matches,
                parent,
                title: step.title().to_string(),
            });
        }
        let mut trace = RouteTrace {
            entity_id: entity.id.raw.to_string(),
            entity_kind: entity.kind.raw.to_string(),
            route: route.name.clone(),
            annotations,
            skipped: route.skipped.iter().map(ToString::to_string).collect(),
            complete: false,
            missing: Vec::new(),
            text: String::new(),
            start,
            steps,
        };
        let kept = trace.kept();
        trace.complete = kept.start;
        trace.missing = trace.missing(&kept);
        trace.text = trace.render(&Kept::all(&trace.steps));
        trace
    }

    /// The thread as the CLI prints it: with [`Partial::No`], a complete thread over its kept matches (dead-end
    /// branches left out) and `None` for an incomplete one; with [`Partial::Yes`], the thread over every match.
    pub fn thread(&self, partial: Partial) -> Option<String> {
        match partial {
            Partial::Yes => Some(self.text.clone()),
            Partial::No if self.complete => Some(self.render(&self.kept())),
            Partial::No => None,
        }
    }

    /// What the CLI prints for an incomplete thread without `--partial`.
    pub fn incomplete_line(&self) -> String {
        let missing: Vec<String> = self
            .missing
            .iter()
            .filter_map(|name| self.steps.iter().find(|s| &s.name == name))
            .map(|step| format!("no {}", step.title()))
            .collect();
        format!(
            "{}: incomplete thread ({}); print it with: {}",
            self.start.name,
            missing.join(", "),
            specforge_common::remedy::command("specforge trace {} --partial", &[&self.entity_id])
        )
    }

    /// The thread in one line, as a code lens titles it: `thread: ` and each step, its one match, the count of its
    /// matches, or `no <step>` when the step it goes on from has a match.
    pub fn summary(&self) -> String {
        let all = Kept::all(&self.steps);
        let parts: Vec<String> = self
            .steps
            .iter()
            .enumerate()
            .filter_map(|(at, step)| match step.matches.as_slice() {
                [one] => Some(one.segment.text.clone()),
                [] => self
                    .parent_has_match(&all, at)
                    .then(|| format!("no {}", step.title())),
                many => Some(format!("{} {}s", many.len(), step.title())),
            })
            .collect();
        format!("thread: {}", parts.join(" → "))
    }

    /// Which matches are kept: those under which every step that goes on from theirs has a kept match.
    fn kept(&self) -> Kept {
        let mut matches: Vec<Vec<bool>> = self
            .steps
            .iter()
            .map(|s| vec![true; s.matches.len()])
            .collect();
        // A step goes on only from an earlier one: from the last step back, each child is settled first.
        for at in (0..self.steps.len()).rev() {
            let kept_under = |parent_at: Option<usize>, matches: &[Vec<bool>]| {
                self.children(Some(at)).all(|child| {
                    self.steps[child]
                        .matches
                        .iter()
                        .enumerate()
                        .any(|(i, m)| m.parent_at == parent_at && matches[child][i])
                })
            };
            for index in 0..self.steps[at].matches.len() {
                matches[at][index] = kept_under(Some(index), &matches);
            }
        }
        let start = self
            .children(None)
            .all(|child| matches[child].iter().any(|&kept| kept));
        Kept { start, matches }
    }

    /// The steps the thread falls short at: reached, with no kept match, though every reached step that goes on from
    /// it has one. A step the one before it never reached is no reason of its own.
    fn missing(&self, kept: &Kept) -> Vec<String> {
        let all = Kept::all(&self.steps);
        let reached = |at: usize| self.parent_has_match(&all, at);
        let lacks = |at: usize| !kept.matches[at].iter().any(|&k| k);
        (0..self.steps.len())
            .filter(|&at| {
                reached(at)
                    && lacks(at)
                    && self
                        .children(Some(at))
                        .all(|child| !reached(child) || !lacks(child))
            })
            .map(|at| self.steps[at].name.clone())
            .collect()
    }

    /// The steps that go on from `parent` (the start for `None`), in declared order.
    fn children(&self, parent: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        (0..self.steps.len()).filter(move |&at| self.steps[at].parent == parent)
    }

    /// The step at `at` goes on from the start, or from a step with a match `kept` counts.
    fn parent_has_match(&self, kept: &Kept, at: usize) -> bool {
        self.steps[at]
            .parent
            .is_none_or(|parent| kept.matches[parent].iter().any(|&k| k))
    }

    /// The thread over the matches `kept` counts: one line when each step has at most one, else a tree.
    fn render(&self, kept: &Kept) -> String {
        let counted = |at: usize| kept.matches[at].iter().filter(|&&k| k).count();
        if (0..self.steps.len()).all(|at| counted(at) <= 1) {
            let mut line = self.start.text.clone();
            for (at, step) in self.steps.iter().enumerate() {
                let one = step
                    .matches
                    .iter()
                    .zip(&kept.matches[at])
                    .find(|(_, kept)| **kept);
                match one {
                    Some((m, _)) => {
                        line.push_str(" → ");
                        line.push_str(&m.segment.text);
                    }
                    None if self.parent_has_match(kept, at) => {
                        line.push_str(&format!(" → no {}", step.title()));
                    }
                    None => {}
                }
            }
            line
        } else {
            let mut lines = vec![self.start.text.clone()];
            self.branches(kept, None, None, 1, &mut lines);
            lines.join("\n")
        }
    }

    /// The lines under the match `parent_at` of the step `parent` (the start for `None`), `depth` deep.
    fn branches(
        &self,
        kept: &Kept,
        parent: Option<usize>,
        parent_at: Option<usize>,
        depth: usize,
        lines: &mut Vec<String>,
    ) {
        let indent = "  ".repeat(depth);
        for child in self.children(parent) {
            let mut any = false;
            for (index, m) in self.steps[child].matches.iter().enumerate() {
                if m.parent_at != parent_at || !kept.matches[child][index] {
                    continue;
                }
                any = true;
                lines.push(format!("{indent}→ {}", m.segment.text));
                self.branches(kept, Some(child), Some(index), depth + 1, lines);
            }
            if !any {
                lines.push(format!("{indent}→ no {}", self.steps[child].title()));
            }
        }
    }
}

/// The matches a rendering counts, step by step, and whether the start is kept.
struct Kept {
    start: bool,
    matches: Vec<Vec<bool>>,
}

impl Kept {
    /// Every match.
    fn all(steps: &[StepTrace]) -> Kept {
        Kept {
            start: true,
            matches: steps.iter().map(|s| vec![true; s.matches.len()]).collect(),
        }
    }
}

impl Segment {
    /// `node` as the thread prints it: the label and a space, its name, then each annotation's text.
    fn of(label: Option<&str>, node: &Node, annotations: &[Annotated]) -> Segment {
        let name = node.name().to_string();
        let mut text = label.map_or_else(String::new, |label| format!("{label} "));
        text.push_str(&name);
        for printed in annotations.iter().filter_map(Annotated::text) {
            text.push(' ');
            text.push_str(printed);
        }
        Segment { name, text }
    }
}

/// What each of `annotations` prints of `node`.
fn annotate(
    view: &ProjectView,
    selection: &Selection,
    annotations: &[RouteAnnotation],
    node: &Node,
) -> Vec<Annotated> {
    annotations
        .iter()
        .map(|annotation| match annotation {
            RouteAnnotation::Field { of, field, absent } => {
                let id = node.id.raw.as_str();
                let read: Vec<String> = match of {
                    Some(chain) => selection
                        .entities(id, chain)
                        .iter()
                        .filter_map(|id| view.graph().node(id))
                        .flat_map(|reached| field_values(view, reached, field))
                        .collect(),
                    None => field_values(view, node, field),
                };
                let mut values: Vec<String> = Vec::with_capacity(read.len());
                for value in read {
                    if !values.contains(&value) {
                        values.push(value);
                    }
                }
                let text = if values.is_empty() {
                    absent.as_ref().map(|absent| format!("({absent})"))
                } else {
                    Some(format!("({})", values.join(", ")))
                };
                Annotated::Field {
                    name: field.clone(),
                    values,
                    text,
                }
            }
            RouteAnnotation::Judgement { code, absent } => {
                let judgement = view.project().judgements().get(node.id.raw.as_str(), code);
                let text = match judgement {
                    Some(j) => Some(match &j.outcome {
                        Some(outcome) => format!("{outcome}, {}", j.state),
                        None => j.state.clone(),
                    }),
                    None => absent.clone(),
                };
                Annotated::Judgement {
                    code: code.clone(),
                    state: judgement.map(|j| j.state.clone()),
                    outcome: judgement.and_then(|j| j.outcome.clone()),
                    text,
                }
            }
        })
        .collect()
}

/// The values of `path` (a field, or `block.member`) on `node`, as written (else as computed): lists flattened,
/// scalars without quotes, a reference as its target's name.
fn field_values(view: &ProjectView, node: &Node, path: &str) -> Vec<String> {
    let graph = view.graph();
    let forms = view.registries().field_forms();
    let (field, rest) = path
        .split_once('.')
        .map_or((path, None), |(f, r)| (f, Some(r)));
    let written = wire::resolved(graph, forms, node, field).map(Value::from);
    let value = match written {
        Some(value) => rest.map_or(value.clone(), |rest| member(&value, rest)),
        None => match view.computed().get(node.id.raw.as_str(), path) {
            Some(computed) => computed.as_value().clone(),
            None => return Vec::new(),
        },
    };
    let reference = matches!(
        forms.reading_at(node.kind.raw.as_str(), path),
        Reading::Declared(form) if form.field_type().is_reference()
    );
    let mut values = Vec::new();
    scalars(&value, &mut values);
    if reference {
        for value in &mut values {
            if let Some(target) = graph.node(value) {
                *value = target.name().to_string();
            }
        }
    }
    values
}

/// The member `rest` (`a.b`) of `value`, through every item of a list on the way.
fn member(value: &Value, rest: &str) -> Value {
    let (name, deeper) = rest
        .split_once('.')
        .map_or((rest, None), |(n, d)| (n, Some(d)));
    let picked = match value {
        Value::Array(items) => Value::Array(items.iter().map(|item| member(item, rest)).collect()),
        Value::Object(fields) => fields.get(name).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    };
    match (deeper, &picked) {
        (_, Value::Array(_)) | (None, _) => picked,
        (Some(deeper), _) => member(&picked, deeper),
    }
}

/// The scalars of `value`, in order: strings without quotes, numbers and booleans as written, lists flattened, an
/// object as its compact JSON.
fn scalars(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::Null => {}
        Value::String(text) => into.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| scalars(item, into)),
        other => into.push(other.to_string()),
    }
}

/// The trace of each of `ids` (each of a kind a route is followed from) along its kind's followed route, with one
/// selection over the whole request: what the LSP's lenses and hovers read. An id the graph lacks, or of a kind no
/// route starts at, is left out.
pub fn route_traces<'i>(
    view: &ProjectView,
    ids: impl IntoIterator<Item = &'i str>,
) -> Vec<RouteTrace> {
    let selection = Selection::of(view.graph(), view.registries());
    let routes = view.registries().routes();
    ids.into_iter()
        .filter_map(|id| {
            let node = view.graph().node(id)?;
            let route = routes.followed(node.kind.raw.as_str())?;
            Some(RouteTrace::of(view, &selection, route, node))
        })
        .collect()
}
