use specforge_ops::stats::{ProjectStats, compute_project_stats};
use specforge_project::coverage::{CoverageRegistries, ProjectCoverage};
use std::path::Path;

use crate::OutputFormat;
use crate::pipeline;

pub fn run(path: &Path, format: OutputFormat) -> i32 {
    let ctx = pipeline::compile(path);
    // The proof percentage reads what `specforge collect` last recorded; a
    // report that is there but unusable is an error, as in `analyze`.
    let report = match specforge_project::coverage::read_report(path) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    // Coverage is the coverage rule's, over the kinds the extensions
    // declare testable, less the entities W004 exempts.
    let coverage = ProjectCoverage::compute(
        &ctx.graph,
        CoverageRegistries {
            kinds: &ctx.kind_registry,
            fields: &ctx.field_registry,
            rules: &ctx.extension_rules,
        },
        report.as_ref(),
    );
    let stats = compute_project_stats(&ctx.graph, &coverage.summary, &ctx.diagnostics);

    match format {
        OutputFormat::Json => print_json(&stats),
        OutputFormat::Human => print_human(&stats),
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

fn print_json(stats: &ProjectStats) {
    let json = serde_json::json!({
        "total_entities": stats.total_entities,
        "total_edges": stats.total_edges,
        "orphan_count": stats.orphan_count,
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
