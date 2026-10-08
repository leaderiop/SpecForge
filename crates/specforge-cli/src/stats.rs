use specforge_ops::stats::Stats;
use specforge_ops::view::ProjectView;
use std::path::Path;

use crate::OutputFormat;
use crate::outcome::{Exit, Refusal};
use crate::pipeline;

/// `specforge stats`: the stats operation over the project compiled at
/// `path`, its proof percentage read from what `specforge collect` last
/// recorded there; a report that is there but unusable is an error (exit
/// 2), as in `analyze`.
pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let stats = match specforge_ops::stats::stats(&ProjectView::of(&project)) {
        Ok(stats) => stats,
        Err(error) => return Refusal::measuring(format).report(&error),
    };

    match format {
        OutputFormat::Json => print_json(&stats),
        OutputFormat::Human => print_human(&stats),
    }

    Exit::Passed.code()
}

fn print_human(stats: &Stats) {
    println!("Entities: {}", stats.total_entities);
    for (kind, count) in &stats.entities_by_kind {
        println!("  {}: {}", kind, count);
    }
    println!("Edges:    {}", stats.total_edges);
    println!("Unconnected: {}", stats.unconnected_count);
    println!("Verified: {}", stats.verified_count);
    println!(
        "Declared: {}% of {} testable",
        stats.declared_pct.round(),
        stats.testable_count
    );
    if let Some(proof_pct) = stats.proof_pct {
        println!(
            "Proven:   {}% of {} testable",
            proof_pct.round(),
            stats.testable_count
        );
    }
    if stats.error_count > 0 || stats.warning_count > 0 || stats.info_count > 0 {
        println!(
            "Diagnostics: {} errors, {} warnings, {} info",
            stats.error_count, stats.warning_count, stats.info_count
        );
    }
}

fn print_json(stats: &Stats) {
    let json = serde_json::json!({
        "total_entities": stats.total_entities,
        "total_edges": stats.total_edges,
        "unconnected_count": stats.unconnected_count,
        "verified_count": stats.verified_count,
        "testable_count": stats.testable_count,
        "declared_count": stats.declared_count,
        "declared_pct": stats.declared_pct,
        "proof_pct": stats.proof_pct,
        // Deprecated alias of declared_pct.
        "coverage_pct": stats.coverage_pct,
        "error_count": stats.error_count,
        "warning_count": stats.warning_count,
        "info_count": stats.info_count,
        "entities_by_kind": stats.entities_by_kind,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&json).expect("serialize JSON output")
    );
}
