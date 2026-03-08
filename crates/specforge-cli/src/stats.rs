use specforge_emitter::{compute_stats_with_diagnostics, ProjectStats};
use specforge_graph::build_graph;
use specforge_resolver::resolve_project;
use std::path::Path;

pub fn run(path: &Path, format: &str) -> i32 {
    let resolved = resolve_project(path);
    let spec_files: Vec<_> = resolved.files.iter().map(|f| f.spec_file.clone()).collect();
    let (graph, build_diagnostics) = build_graph(&spec_files);

    let mut all_diagnostics = resolved.diagnostics;
    all_diagnostics.extend(build_diagnostics);

    let stats = compute_stats_with_diagnostics(&graph, &[], &all_diagnostics);

    match format {
        "json" => print_json(&stats),
        _ => print_human(&stats),
    }

    0
}

fn print_human(stats: &ProjectStats) {
    println!("Entities: {}", stats.total_entities);
    for (kind, count) in &stats.entities_by_kind {
        println!("  {}: {}", kind, count);
    }
    println!("Edges:    {}", stats.total_edges);
    println!("Orphans:  {}", stats.orphan_count);
    println!("Verified: {}", stats.verified_count);
    if stats.error_count > 0 || stats.warning_count > 0 || stats.info_count > 0 {
        println!("Diagnostics: {} errors, {} warnings, {} info",
            stats.error_count, stats.warning_count, stats.info_count);
    }
}

fn print_json(stats: &ProjectStats) {
    let json = serde_json::json!({
        "total_entities": stats.total_entities,
        "total_edges": stats.total_edges,
        "orphan_count": stats.orphan_count,
        "verified_count": stats.verified_count,
        "testable_count": stats.testable_count,
        "coverage_pct": stats.coverage_pct,
        "error_count": stats.error_count,
        "warning_count": stats.warning_count,
        "info_count": stats.info_count,
        "entities_by_kind": stats.entities_by_kind,
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
