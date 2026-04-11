use crate::pipeline;
use super::{
    list_entities, ListFilter,
    milestone_completion, journey_coverage, feature_impact, feature_dependents,
    persona_features, channel_features, bulk_status,
    project_health,
};
use std::path::Path;

pub fn run_list(path: &Path, kind: &str, status: Option<&str>, priority: Option<&str>, limit: Option<usize>, offset: Option<usize>, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    let filter = ListFilter {
        kind: kind.to_string(),
        status: status.map(|s| s.to_string()),
        priority: priority.map(|s| s.to_string()),
        limit,
        offset,
    };

    let result = list_entities(&ctx.graph, &filter);

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&result).unwrap_or_default();
            println!("{}", json);
        }
        _ => {
            println!("{} {} entities (showing {}):", result.total, kind, result.entities.len());
            for entity in &result.entities {
                let status_str = entity.status.as_deref().unwrap_or("-");
                let priority_str = entity.priority.as_deref().unwrap_or("-");
                let title = entity.title.as_deref().unwrap_or("");
                println!(
                    "  {} {} [{}] pri={} in={} out={}",
                    entity.id, title, status_str, priority_str,
                    entity.incoming_edges, entity.outgoing_edges
                );
            }
        }
    }
    0
}

pub fn run_milestone_completion(path: &Path, milestone_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match milestone_completion(&ctx.graph, milestone_id) {
        Some(result) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Milestone: {} ({})", result.milestone_id, result.status.as_deref().unwrap_or("-"));
                    println!("Completion: {:.0}% ({}/{} features done)", result.completion_pct, result.done_features, result.total_features);
                    for f in &result.features {
                        println!("  {} [{}]", f.id, f.status.as_deref().unwrap_or("-"));
                    }
                }
            }
            0
        }
        None => {
            eprintln!("milestone '{}' not found", milestone_id);
            1
        }
    }
}

pub fn run_journey_coverage(path: &Path, journey_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match journey_coverage(&ctx.graph, journey_id) {
        Some(result) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Journey: {} (persona: {})", result.journey_id, result.persona.as_deref().unwrap_or("-"));
                    println!("Coverage: {:.0}% ({}/{} features covered by modules)", result.coverage_pct, result.covered_by_modules, result.total_features);
                }
            }
            0
        }
        None => {
            eprintln!("journey '{}' not found", journey_id);
            1
        }
    }
}

pub fn run_feature_impact(path: &Path, feature_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match feature_impact(&ctx.graph, feature_id) {
        Some(result) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&result).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Feature: {}", result.feature_id);
                    println!("  Journeys: {:?}", result.referenced_by_journeys);
                    println!("  Milestones: {:?}", result.referenced_by_milestones);
                    println!("  Modules: {:?}", result.referenced_by_modules);
                    println!("  Depends on: {:?}", result.depends_on);
                    println!("  Depended on by: {:?}", result.depended_on_by);
                }
            }
            0
        }
        None => {
            eprintln!("feature '{}' not found", feature_id);
            1
        }
    }
}

pub fn run_feature_dependents(path: &Path, feature_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match feature_dependents(&ctx.graph, feature_id) {
        Some(deps) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&deps).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Features depending on '{}':", feature_id);
                    for dep in &deps {
                        println!("  {}", dep);
                    }
                    if deps.is_empty() {
                        println!("  (none)");
                    }
                }
            }
            0
        }
        None => {
            eprintln!("feature '{}' not found", feature_id);
            1
        }
    }
}

pub fn run_persona_features(path: &Path, persona_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match persona_features(&ctx.graph, persona_id) {
        Some(features) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&features).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Features for persona '{}':", persona_id);
                    for f in &features {
                        println!("  {}", f);
                    }
                    if features.is_empty() {
                        println!("  (none)");
                    }
                }
            }
            0
        }
        None => {
            eprintln!("persona '{}' not found", persona_id);
            1
        }
    }
}

pub fn run_channel_features(path: &Path, channel_id: &str, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    match channel_features(&ctx.graph, channel_id) {
        Some(features) => {
            match format {
                "json" => {
                    let json = serde_json::to_string_pretty(&features).unwrap_or_default();
                    println!("{}", json);
                }
                _ => {
                    println!("Features for channel '{}':", channel_id);
                    for f in &features {
                        println!("  {}", f);
                    }
                    if features.is_empty() {
                        println!("  (none)");
                    }
                }
            }
            0
        }
        None => {
            eprintln!("channel '{}' not found", channel_id);
            1
        }
    }
}

pub fn run_bulk_status(path: &Path, format: &str) -> i32 {
    let ctx = pipeline::compile(path);
    let results = bulk_status(&ctx.graph);

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&results).unwrap_or_default();
            println!("{}", json);
        }
        _ => {
            for bs in &results {
                println!("{} ({} total):", bs.kind, bs.total);
                for sc in &bs.by_status {
                    println!("  {}: {}", sc.status, sc.count);
                }
            }
            if results.is_empty() {
                println!("No status-bearing entities found.");
            }
        }
    }
    0
}

pub fn run_health(path: &Path, format: &str) -> i32 {
    let ctx = pipeline::compile(path);
    let report = project_health(&ctx.graph);

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            println!("{}", json);
        }
        _ => {
            println!("Project Health Score: {:.0}/100", report.score.overall);
            println!("  Coverage:     {:.0}%", report.score.coverage);
            println!("  Connectivity: {:.0}%", report.score.connectivity);
            println!("  Completeness: {:.0}%", report.score.completeness);
            println!();
            println!("Entity counts:");
            for ec in &report.entity_counts {
                if ec.count > 0 {
                    println!("  {}: {}", ec.kind, ec.count);
                }
            }
            if !report.orphan_counts.is_empty() {
                println!();
                println!("Orphan entities:");
                for oc in &report.orphan_counts {
                    if oc.orphans > 0 {
                        println!("  {}: {}/{}", oc.kind, oc.orphans, oc.total);
                    }
                }
            }
        }
    }
    0
}
