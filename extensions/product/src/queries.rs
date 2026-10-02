//! Product queries: pure reads of the graph the host passes a command.
//!
//! Each one answers a planning question over this extension's own kinds and
//! the fields they declare (`status`, `priority`, `features`, ...). The
//! host knows none of them; it runs the `cmd__product_*` exports in
//! `commands.rs`, which render these results.

use serde::Serialize;
use specforge_extension_sdk::prelude::{CommandGraph, GraphNode};
use std::collections::BTreeMap;

/// The kinds this extension declares, in the order `health` reports them.
pub const PRODUCT_KINDS: &[&str] = &[
    "feature",
    "journey",
    "deliverable",
    "milestone",
    "module",
    "term",
    "persona",
    "channel",
    "release",
];

/// The kinds with a lifecycle `status`, in the order `bulk-status` reports
/// them.
const STATUS_KINDS: &[&str] = &[
    "feature",
    "milestone",
    "deliverable",
    "persona",
    "channel",
    "release",
];

// ── Listing ────────────────────────────────────────────────────────────────

/// Which entities a list command returns: one kind, optionally narrowed by
/// status and priority, then paged.
#[derive(Debug, Default)]
pub struct ListFilter<'a> {
    pub kind: &'a str,
    pub status: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListResult {
    pub entities: Vec<ListEntity>,
    /// Matching entities before paging.
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ListEntity {
    pub id: String,
    pub title: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    pub incoming_edges: usize,
    pub outgoing_edges: usize,
}

pub fn list_entities(graph: &CommandGraph, filter: &ListFilter) -> ListResult {
    let matches = |node: &&GraphNode, field: &str, wanted: Option<&str>| {
        wanted.is_none_or(|w| node.text(field) == Some(w))
    };
    let mut entities: Vec<ListEntity> = graph
        .nodes_of_kind(filter.kind)
        .filter(|n| matches(n, "status", filter.status))
        .filter(|n| matches(n, "priority", filter.priority))
        .map(|n| ListEntity {
            id: n.id.clone(),
            title: n.title.clone(),
            kind: n.kind.clone(),
            status: text(n, "status"),
            priority: text(n, "priority"),
            incoming_edges: graph.edges_to(&n.id).len(),
            outgoing_edges: graph.edges_from(&n.id).len(),
        })
        .collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    let total = entities.len();
    let entities = entities
        .into_iter()
        .skip(filter.offset.unwrap_or(0))
        .take(filter.limit.unwrap_or(usize::MAX))
        .collect();
    ListResult { entities, total }
}

// ── Milestones and journeys ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MilestoneCompletion {
    pub milestone_id: String,
    pub total_features: usize,
    pub done_features: usize,
    pub completion_pct: f64,
    pub status: Option<String>,
    pub features: Vec<FeatureStatus>,
}

#[derive(Debug, Serialize)]
pub struct FeatureStatus {
    pub id: String,
    pub status: Option<String>,
}

/// The milestone's features (in declaration order) and how many are done;
/// `None` when `milestone_id` is not a milestone.
pub fn milestone_completion(
    graph: &CommandGraph,
    milestone_id: &str,
) -> Option<MilestoneCompletion> {
    let node = of_kind(graph, milestone_id, "milestone")?;
    // The referenced features, in the order the milestone lists them.
    let mut referenced = targets(graph, milestone_id, "features");
    let declared = node.list("features");
    referenced.sort_by_key(|id| declared.iter().position(|d| d == id).unwrap_or(usize::MAX));
    let features: Vec<FeatureStatus> = referenced
        .iter()
        .filter_map(|id| graph.node(id))
        .map(|f| FeatureStatus {
            id: f.id.clone(),
            status: text(f, "status"),
        })
        .collect();
    let done = features
        .iter()
        .filter(|f| f.status.as_deref() == Some("done"))
        .count();
    Some(MilestoneCompletion {
        milestone_id: milestone_id.to_string(),
        total_features: features.len(),
        done_features: done,
        completion_pct: pct(done, features.len()),
        status: text(node, "status"),
        features,
    })
}

#[derive(Debug, Serialize)]
pub struct JourneyCoverage {
    pub journey_id: String,
    pub persona: Option<String>,
    pub total_features: usize,
    pub covered_by_modules: usize,
    pub coverage_pct: f64,
}

/// How many of the journey's features some module contains; `None` when
/// `journey_id` is not a journey.
pub fn journey_coverage(graph: &CommandGraph, journey_id: &str) -> Option<JourneyCoverage> {
    let node = of_kind(graph, journey_id, "journey")?;
    let features = targets(graph, journey_id, "features");
    let covered = features
        .iter()
        .filter(|f| {
            graph
                .edges_to(f)
                .iter()
                .any(|e| kind_of(graph, &e.source) == Some("module"))
        })
        .count();
    Some(JourneyCoverage {
        journey_id: journey_id.to_string(),
        persona: text(node, "persona"),
        total_features: features.len(),
        covered_by_modules: covered,
        coverage_pct: pct(covered, features.len()),
    })
}

// ── Features ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FeatureImpact {
    pub feature_id: String,
    pub referenced_by_journeys: Vec<String>,
    pub referenced_by_milestones: Vec<String>,
    pub referenced_by_modules: Vec<String>,
    pub depends_on: Vec<String>,
    pub depended_on_by: Vec<String>,
}

/// What references the feature, by kind, and what it depends on; `None`
/// when `feature_id` is not a feature.
pub fn feature_impact(graph: &CommandGraph, feature_id: &str) -> Option<FeatureImpact> {
    of_kind(graph, feature_id, "feature")?;
    let mut impact = FeatureImpact {
        feature_id: feature_id.to_string(),
        referenced_by_journeys: Vec::new(),
        referenced_by_milestones: Vec::new(),
        referenced_by_modules: Vec::new(),
        depends_on: targets(graph, feature_id, "depends_on"),
        depended_on_by: Vec::new(),
    };
    for edge in graph.edges_to(feature_id) {
        let list = match kind_of(graph, &edge.source) {
            Some("journey") => &mut impact.referenced_by_journeys,
            Some("milestone") => &mut impact.referenced_by_milestones,
            Some("module") => &mut impact.referenced_by_modules,
            Some("feature") => &mut impact.depended_on_by,
            _ => continue,
        };
        list.push(edge.source.clone());
    }
    Some(impact)
}

/// The entities that declare `depends_on` the feature; `None` when there is
/// no entity `feature_id`.
pub fn feature_dependents(graph: &CommandGraph, feature_id: &str) -> Option<Vec<String>> {
    graph.node(feature_id)?;
    Some(
        graph
            .edges_to(feature_id)
            .iter()
            .filter(|e| e.label == "depends_on")
            .map(|e| e.source.clone())
            .collect(),
    )
}

/// The features of every journey the persona undertakes, sorted; `None`
/// when `persona_id` is not a persona.
pub fn persona_features(graph: &CommandGraph, persona_id: &str) -> Option<Vec<String>> {
    of_kind(graph, persona_id, "persona")?;
    let journeys = graph.nodes_of_kind("journey").filter(|j| {
        j.text("persona") == Some(persona_id)
            || graph
                .edges_from(&j.id)
                .iter()
                .any(|e| e.label == "persona" && e.target == persona_id)
    });
    Some(journey_features(graph, journeys.map(|j| j.id.as_str())))
}

/// The features of every journey that uses the channel, sorted; `None`
/// when `channel_id` is not a channel.
pub fn channel_features(graph: &CommandGraph, channel_id: &str) -> Option<Vec<String>> {
    of_kind(graph, channel_id, "channel")?;
    let journeys = graph
        .edges_to(channel_id)
        .into_iter()
        .filter(|e| e.label == "channels" && kind_of(graph, &e.source) == Some("journey"))
        .map(|e| e.source.as_str());
    Some(journey_features(graph, journeys))
}

fn journey_features<'a>(
    graph: &CommandGraph,
    journeys: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let mut features: Vec<String> = Vec::new();
    for journey in journeys {
        for feature in targets(graph, journey, "features") {
            if !features.contains(&feature) {
                features.push(feature);
            }
        }
    }
    features.sort();
    features
}

// ── Project-wide ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BulkStatus {
    pub kind: String,
    pub total: usize,
    pub by_status: Vec<StatusCount>,
}

#[derive(Debug, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: usize,
}

/// Per lifecycle kind present in the graph, how many entities have each
/// status (`(none)` for those without one).
pub fn bulk_status(graph: &CommandGraph) -> Vec<BulkStatus> {
    STATUS_KINDS
        .iter()
        .filter_map(|&kind| {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut total = 0;
            for node in graph.nodes_of_kind(kind) {
                total += 1;
                let status = text(node, "status").unwrap_or_else(|| "(none)".to_string());
                *counts.entry(status).or_insert(0) += 1;
            }
            (total > 0).then(|| BulkStatus {
                kind: kind.to_string(),
                total,
                by_status: counts
                    .into_iter()
                    .map(|(status, count)| StatusCount { status, count })
                    .collect(),
            })
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub score: HealthScore,
    pub entity_counts: Vec<EntityCount>,
    pub orphan_counts: Vec<OrphanCount>,
    pub completeness: CompletenessReport,
}

#[derive(Debug, Serialize)]
pub struct HealthScore {
    pub overall: f64,
    pub coverage: f64,
    pub connectivity: f64,
    pub completeness: f64,
}

#[derive(Debug, Serialize)]
pub struct EntityCount {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct OrphanCount {
    pub kind: String,
    pub orphans: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct CompletenessReport {
    pub features_with_status: usize,
    pub features_total: usize,
    pub milestones_with_features: usize,
    pub milestones_total: usize,
}

/// A 0-100 score averaging coverage (product entities something references),
/// connectivity (edge density) and completeness (features with a status,
/// milestones with references).
pub fn project_health(graph: &CommandGraph) -> HealthReport {
    let mut entity_counts = Vec::new();
    let mut orphan_counts = Vec::new();
    let (mut total_entities, mut total_orphans) = (0usize, 0usize);
    for &kind in PRODUCT_KINDS {
        let count = graph.nodes_of_kind(kind).count();
        let orphans = graph
            .nodes_of_kind(kind)
            .filter(|n| graph.edges_to(&n.id).is_empty())
            .count();
        entity_counts.push(EntityCount {
            kind: kind.to_string(),
            count,
        });
        if count > 0 {
            orphan_counts.push(OrphanCount {
                kind: kind.to_string(),
                orphans,
                total: count,
            });
        }
        total_entities += count;
        total_orphans += orphans;
    }

    let coverage = if total_entities > 0 {
        pct(total_entities - total_orphans, total_entities)
    } else {
        100.0
    };
    let connectivity = if total_entities > 1 {
        let max_edges = total_entities * (total_entities - 1);
        (graph.edges().len() as f64 / max_edges as f64).min(1.0) * 100.0
    } else {
        100.0
    };

    let features_total = graph.nodes_of_kind("feature").count();
    let features_with_status = graph
        .nodes_of_kind("feature")
        .filter(|n| n.has_field("status"))
        .count();
    let milestones_total = graph.nodes_of_kind("milestone").count();
    let milestones_with_features = graph
        .nodes_of_kind("milestone")
        .filter(|n| !graph.edges_from(&n.id).is_empty())
        .count();
    let completeness = if features_total + milestones_total > 0 {
        pct(
            features_with_status + milestones_with_features,
            features_total + milestones_total,
        )
    } else {
        100.0
    };

    HealthReport {
        score: HealthScore {
            overall: (coverage + connectivity + completeness) / 3.0,
            coverage,
            connectivity,
            completeness,
        },
        entity_counts,
        orphan_counts,
        completeness: CompletenessReport {
            features_with_status,
            features_total,
            milestones_with_features,
            milestones_total,
        },
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn text(node: &GraphNode, field: &str) -> Option<String> {
    node.text(field).map(str::to_string)
}

fn of_kind<'a>(graph: &'a CommandGraph, id: &str, kind: &str) -> Option<&'a GraphNode> {
    graph.node(id).filter(|n| n.kind == kind)
}

fn kind_of<'a>(graph: &'a CommandGraph, id: &str) -> Option<&'a str> {
    graph.node(id).map(|n| n.kind.as_str())
}

/// The targets of `id`'s references declared in `label`.
fn targets(graph: &CommandGraph, id: &str, label: &str) -> Vec<String> {
    graph
        .edges_from(id)
        .iter()
        .filter(|e| e.label == label)
        .map(|e| e.target.clone())
        .collect()
}

fn pct(part: usize, whole: usize) -> f64 {
    if whole > 0 {
        (part as f64 / whole as f64) * 100.0
    } else {
        0.0
    }
}
