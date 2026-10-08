//! The exploration: where to start reading a project's graph, a read view
//! over the project view (ADR 0015, "Prompt read views"). The explore
//! prompt and `specforge explore` render it.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_graph::Graph;

use crate::OpError;
use crate::view::ProjectView;

/// How many starting points an exploration names.
pub const STARTING_POINTS: usize = 5;
/// How many of the most connected entities an exploration names.
pub const MOST_CONNECTED: usize = 10;

/// What to explore.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExplorationRequest<'a> {
    /// Explore from this entity: the selection is what it reaches.
    pub entity_id: Option<&'a str>,
    /// Only entities of this kind. A kind the project does not know selects
    /// nothing and is an I020 notice (`KnownKinds::unknown_in`).
    pub kind: Option<&'a str>,
    /// Hops from `entity_id`; unbounded when `None`; ignored without
    /// `entity_id`.
    pub depth: Option<usize>,
}

/// The path from the exploration's entity to one entity it reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipPath {
    pub to: String,
    /// The labels of the edges on the path, from the entity on
    /// (`Graph::reach_path`).
    pub labels: Vec<String>,
}

/// What an exploration answers. Every list is about the selected entities:
/// those `entity_id` reaches within `depth` (every entity without it), of
/// `kind` when given. Degrees count every edge to or from another entity of
/// the project, selected or not.
#[derive(Debug, Clone, PartialEq)]
pub struct Exploration {
    /// The entity explored from.
    pub from: Option<String>,
    /// The selected entities, in id order.
    pub selected: Vec<String>,
    /// From `from` to each selected entity it reaches (`from` aside),
    /// nearest first.
    pub paths: Vec<RelationshipPath>,
    /// The selected connected entities with the highest lead, ties by id,
    /// at most [`STARTING_POINTS`].
    pub starting_points: Vec<String>,
    /// The selected connected entities with the most edges, ties by id, at
    /// most [`MOST_CONNECTED`].
    pub most_connected: Vec<String>,
    /// The selected unconnected entities, in id order.
    pub unconnected: Vec<String>,
    /// I020 for a kind the project does not know.
    pub notices: Vec<Diagnostic>,
}

impl Exploration {
    /// The exploration as the explore prompt's payload and `specforge
    /// explore --format json` write it (`McpExplorePromptResult`):
    /// `matching_entities`, `relationship_paths` (`from_entity`,
    /// `to_entity`, `edge_types`, `path_length`), `starting_points`,
    /// `high_connectivity`, `unconnected`, `notices` (the diagnostics JSON,
    /// `specforge_common::diagnostics_json`).
    pub fn to_json(&self) -> Value {
        let from = self.from.as_deref().unwrap_or_default();
        let paths: Vec<Value> = self
            .paths
            .iter()
            .map(|path| {
                json!({
                    "from_entity": from,
                    "to_entity": path.to,
                    "edge_types": path.labels,
                    "path_length": path.labels.len(),
                })
            })
            .collect();
        json!({
            "matching_entities": self.selected,
            "relationship_paths": paths,
            "starting_points": self.starting_points,
            "high_connectivity": self.most_connected,
            "unconnected": self.unconnected,
            "notices": specforge_common::diagnostics_json(&self.notices),
        })
    }
}

/// The exploration `request` asks for. An `entity_id` the graph lacks is
/// `navigate::not_found` (E003, did-you-mean).
pub fn explore(view: &ProjectView, request: &ExplorationRequest) -> Result<Exploration, OpError> {
    let graph = view.graph();
    let notices = request
        .kind
        .map(|kind| view.kinds().unknown_in(&[kind]))
        .unwrap_or_default();

    let reached = request
        .entity_id
        .map(|id| view.neighbourhood(id, request.depth))
        .transpose()?;
    let of_kind = |id: &str| {
        request
            .kind
            .is_none_or(|kind| graph.node(id).is_some_and(|node| node.kind.raw == kind))
    };
    let selection: BTreeSet<&'static str> = match &reached {
        Some(reached) => reached
            .iter()
            .map(|r| r.id.as_str())
            .filter(|id| of_kind(id))
            .collect(),
        None => graph
            .nodes()
            .into_iter()
            .map(|node| node.id.raw.as_str())
            .filter(|id| of_kind(id))
            .collect(),
    };

    let connectivity = view.connectivity();
    let mut connected: Vec<(&'static str, _)> = connectivity
        .iter()
        .filter(|(id, degree)| selection.contains(id) && !degree.is_unconnected())
        .collect();
    connected.sort_by_key(|(id, degree)| (Reverse(degree.lead()), *id));
    let starting_points = connected
        .iter()
        .take(STARTING_POINTS)
        .map(|(id, _)| id.to_string())
        .collect();
    connected.sort_by_key(|(id, degree)| (Reverse(degree.total()), *id));
    let most_connected = connected
        .iter()
        .take(MOST_CONNECTED)
        .map(|(id, _)| id.to_string())
        .collect();
    let unconnected = connectivity
        .unconnected()
        .filter(|id| selection.contains(id))
        .map(str::to_string)
        .collect();

    let paths = reached
        .as_deref()
        .map(|reached| {
            reached
                .iter()
                .skip(1)
                .filter(|r| selection.contains(r.id.as_str()))
                .map(|r| RelationshipPath {
                    to: r.id.as_str().to_string(),
                    labels: Graph::reach_path(reached, r.id)
                        .iter()
                        .map(|label| label.as_str().to_string())
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Exploration {
        from: request.entity_id.map(str::to_string),
        selected: selection.into_iter().map(str::to_string).collect(),
        paths,
        starting_points,
        most_connected,
        unconnected,
        notices,
    })
}
