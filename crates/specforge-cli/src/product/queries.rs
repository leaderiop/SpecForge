use serde::Serialize;
use specforge_graph::Graph;

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

pub fn milestone_completion(graph: &Graph, milestone_id: &str) -> Option<MilestoneCompletion> {
    let node = graph.node(milestone_id)?;
    if node.kind.raw != "milestone" {
        return None;
    }

    let status = get_field(node, "status");
    let feature_edges = graph.edges_from(milestone_id);
    let feature_ids: Vec<String> = feature_edges
        .iter()
        .filter(|e| e.label == "features")
        .map(|e| e.target.to_string())
        .collect();

    let mut features = Vec::new();
    let mut done_count = 0;
    for fid in &feature_ids {
        if let Some(fnode) = graph.node(fid.as_str()) {
            let fs = get_field(fnode, "status");
            if fs.as_deref() == Some("done") {
                done_count += 1;
            }
            features.push(FeatureStatus {
                id: fid.to_string(),
                status: fs,
            });
        }
    }

    let total = features.len();
    let pct = if total > 0 {
        (done_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Some(MilestoneCompletion {
        milestone_id: milestone_id.to_string(),
        total_features: total,
        done_features: done_count,
        completion_pct: pct,
        status,
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

pub fn journey_coverage(graph: &Graph, journey_id: &str) -> Option<JourneyCoverage> {
    let node = graph.node(journey_id)?;
    if node.kind.raw != "journey" {
        return None;
    }

    let persona = get_field(node, "persona");
    let feature_edges: Vec<String> = graph
        .edges_from(journey_id)
        .iter()
        .filter(|e| e.label == "features")
        .map(|e| e.target.to_string())
        .collect();

    let total = feature_edges.len();
    let mut covered = 0;
    for fid in &feature_edges {
        // Check if any module references this feature
        let has_module = graph.edges_to(fid.as_str()).iter().any(|e| {
            graph.node(e.source.as_str()).is_some_and(|n| n.kind.raw == "module")
        });
        if has_module {
            covered += 1;
        }
    }

    let pct = if total > 0 {
        (covered as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Some(JourneyCoverage {
        journey_id: journey_id.to_string(),
        persona,
        total_features: total,
        covered_by_modules: covered,
        coverage_pct: pct,
    })
}

#[derive(Debug, Serialize)]
pub struct FeatureImpact {
    pub feature_id: String,
    pub referenced_by_journeys: Vec<String>,
    pub referenced_by_milestones: Vec<String>,
    pub referenced_by_modules: Vec<String>,
    pub depends_on: Vec<String>,
    pub depended_on_by: Vec<String>,
}

pub fn feature_impact(graph: &Graph, feature_id: &str) -> Option<FeatureImpact> {
    let node = graph.node(feature_id)?;
    if node.kind.raw != "feature" {
        return None;
    }

    let incoming = graph.edges_to(feature_id);
    let mut journeys = Vec::new();
    let mut milestones = Vec::new();
    let mut modules = Vec::new();
    let mut depended_on_by = Vec::new();

    for edge in &incoming {
        if let Some(source_node) = graph.node(edge.source.as_str()) {
            match source_node.kind.raw.as_str() {
                "journey" => journeys.push(edge.source.to_string()),
                "milestone" => milestones.push(edge.source.to_string()),
                "module" => modules.push(edge.source.to_string()),
                "feature" => depended_on_by.push(edge.source.to_string()),
                _ => {}
            }
        }
    }

    let depends_on: Vec<String> = graph
        .edges_from(feature_id)
        .iter()
        .filter(|e| e.label == "depends_on")
        .map(|e| e.target.to_string())
        .collect();

    Some(FeatureImpact {
        feature_id: feature_id.to_string(),
        referenced_by_journeys: journeys,
        referenced_by_milestones: milestones,
        referenced_by_modules: modules,
        depends_on,
        depended_on_by,
    })
}

pub fn feature_dependents(graph: &Graph, feature_id: &str) -> Option<Vec<String>> {
    graph.node(feature_id)?;
    let deps: Vec<String> = graph
        .edges_to(feature_id)
        .iter()
        .filter(|e| e.label == "depends_on")
        .map(|e| e.source.to_string())
        .collect();
    Some(deps)
}

pub fn persona_features(graph: &Graph, persona_id: &str) -> Option<Vec<String>> {
    let node = graph.node(persona_id)?;
    if node.kind.raw != "persona" {
        return None;
    }

    // Find journeys that reference this persona (via identifier field or edge), then collect their features
    let mut features = Vec::new();
    for journey in graph.nodes() {
        if journey.kind.raw != "journey" {
            continue;
        }
        let refs_persona = get_field(journey, "persona").is_some_and(|v| v == persona_id)
            || graph.edges_from(journey.id.raw.as_str()).iter().any(|e| e.label == "persona" && e.target == persona_id);
        if refs_persona {
            for fe in graph.edges_from(journey.id.raw.as_str()) {
                let target_str = fe.target.to_string();
                if fe.label == "features" && !features.contains(&target_str) {
                    features.push(target_str);
                }
            }
        }
    }
    features.sort();
    Some(features)
}

pub fn channel_features(graph: &Graph, channel_id: &str) -> Option<Vec<String>> {
    let node = graph.node(channel_id)?;
    if node.kind.raw != "channel" {
        return None;
    }

    // Find journeys that reference this channel, then collect their features
    let incoming = graph.edges_to(channel_id);
    let mut features = Vec::new();
    for edge in &incoming {
        if edge.label == "channels"
            && let Some(journey) = graph.node(edge.source.as_str())
            && journey.kind.raw == "journey"
        {
            for fe in graph.edges_from(edge.source.as_str()) {
                let target_str = fe.target.to_string();
                if fe.label == "features" && !features.contains(&target_str) {
                    features.push(target_str);
                }
            }
        }
    }
    features.sort();
    Some(features)
}

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

pub fn bulk_status(graph: &Graph) -> Vec<BulkStatus> {
    let kinds = ["feature", "milestone", "deliverable", "persona", "channel", "release"];
    let mut results = Vec::new();

    for kind in &kinds {
        let mut status_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let mut total = 0;
        for node in graph.nodes() {
            if node.kind.raw == *kind {
                total += 1;
                let status = get_field(node, "status").unwrap_or_else(|| "(none)".to_string());
                *status_map.entry(status).or_insert(0) += 1;
            }
        }
        if total > 0 {
            results.push(BulkStatus {
                kind: kind.to_string(),
                total,
                by_status: status_map
                    .into_iter()
                    .map(|(status, count)| StatusCount { status, count })
                    .collect(),
            });
        }
    }
    results
}

fn get_field(node: &specforge_graph::Node, field_name: &str) -> Option<String> {
    node.fields.entries().iter().find_map(|e| {
        if e.key == field_name {
            match &e.value {
                specforge_parser::FieldValue::String(s) => Some(s.clone()),
                specforge_parser::FieldValue::Identifier(s) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    })
}
