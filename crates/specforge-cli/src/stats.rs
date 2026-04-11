use specforge_emitter::{compute_stats_with_diagnostics, ProjectStats};
use std::path::Path;

use crate::pipeline;

pub fn run(path: &Path, format: &str) -> i32 {
    let ctx = pipeline::compile(path);

    let stats = compute_stats_with_diagnostics(&ctx.graph, &[], &ctx.diagnostics);

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
    println!("{}", serde_json::to_string_pretty(&json).expect("serialize JSON output"));
}
