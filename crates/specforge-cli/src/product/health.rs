use serde::Serialize;
use specforge_graph::Graph;

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

const PRODUCT_KINDS: &[&str] = &[
    "feature", "journey", "deliverable", "milestone", "module", "term", "persona", "channel", "release",
];

pub fn project_health(graph: &Graph) -> HealthReport {
    let mut entity_counts = Vec::new();
    let mut orphan_counts = Vec::new();
    let mut total_entities = 0usize;
    let mut total_orphans = 0usize;

    for &kind in PRODUCT_KINDS {
        let nodes: Vec<_> = graph.nodes().into_iter().filter(|n| n.kind.raw == kind).collect();
        let count = nodes.len();
        let orphans = nodes.iter().filter(|n| graph.edges_to(n.id.raw.as_str()).is_empty()).count();

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

    // Coverage: how many entities have at least one edge
    let coverage = if total_entities > 0 {
        ((total_entities - total_orphans) as f64 / total_entities as f64) * 100.0
    } else {
        100.0
    };

    // Connectivity: edge density
    let total_edges = graph.edges().len();
    let connectivity = if total_entities > 1 {
        let max_edges = total_entities * (total_entities - 1);
        (total_edges as f64 / max_edges as f64).min(1.0) * 100.0
    } else {
        100.0
    };

    // Completeness
    let features: Vec<_> = graph.nodes().into_iter().filter(|n| n.kind.raw == "feature").collect();
    let features_total = features.len();
    let features_with_status = features.iter().filter(|n| {
        n.fields.entries().iter().any(|e| e.key == "status")
    }).count();

    let milestones: Vec<_> = graph.nodes().into_iter().filter(|n| n.kind.raw == "milestone").collect();
    let milestones_total = milestones.len();
    let milestones_with_features = milestones.iter().filter(|n| {
        !graph.edges_from(n.id.raw.as_str()).is_empty()
    }).count();

    let completeness_score = if features_total + milestones_total > 0 {
        let num = features_with_status + milestones_with_features;
        let den = features_total + milestones_total;
        (num as f64 / den as f64) * 100.0
    } else {
        100.0
    };

    let overall = (coverage + connectivity + completeness_score) / 3.0;

    HealthReport {
        score: HealthScore {
            overall,
            coverage,
            connectivity,
            completeness: completeness_score,
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
