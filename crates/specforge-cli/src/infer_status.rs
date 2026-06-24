use std::path::Path;

use specforge_common::inference;
use specforge_common::inference::discovery::SourceDiscoveryConfig;
use specforge_common::AnalyzerConfig;
use specforge_emitter::scanner_dispatch;

pub fn run(path: &Path, format: &str, show_gaps: bool, show_stale: bool, show_gaps_detail: bool) -> i32 {
    let manifest = match inference::load_inference_manifest(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}", e);
            return 1;
        }
    };

    let ctx = crate::pipeline::compile(path);
    let analyzer_configs: Vec<AnalyzerConfig> = ctx.manifests.iter()
        .flat_map(|m| m.analyzer_contributions.iter())
        .map(|ac| AnalyzerConfig {
            language: ac.language.clone(),
            file_extensions: ac.file_extensions.clone(),
            excluded_dirs: ac.excluded_dirs.clone(),
        })
        .collect();
    let discovery_config = SourceDiscoveryConfig::from_analyzer_configs(&analyzer_configs);
    let source_files = inference::discover_source_files(path, &manifest.source_roots, &discovery_config);
    let index_map = manifest.source_index_map();

    let unanalyzed: Vec<&str> = source_files
        .iter()
        .filter(|f| !index_map.contains_key(f.as_str()))
        .map(|f| f.as_str())
        .collect();

    let (stale, deleted) = inference::detect_stale_entries(path, &manifest);
    let summary = manifest.compute_summary(source_files.len());

    match format {
        "json" => {
            let json = serde_json::json!({
                "summary": {
                    "files_total": summary.files_total,
                    "files_analyzed": summary.files_analyzed,
                    "entities_produced": summary.entities_produced,
                },
                "unanalyzed": unanalyzed,
                "stale": stale,
                "deleted": deleted,
            });
            println!("{}", serde_json::to_string_pretty(&json).expect("serialize JSON output"));
        }
        _ => {
            let pct = if summary.files_total > 0 {
                (summary.files_analyzed as f64 / summary.files_total as f64) * 100.0
            } else {
                0.0
            };

            println!("Inference Progress");
            println!("  Files:    {}/{} ({:.0}%)", summary.files_analyzed, summary.files_total, pct);
            println!("  Entities: {}", summary.entities_produced);

            if !stale.is_empty() || !deleted.is_empty() {
                println!("  Stale:    {}", stale.len());
                println!("  Deleted:  {}", deleted.len());
            }

            if show_gaps && !unanalyzed.is_empty() {
                println!();
                println!("Unanalyzed files:");
                let mut by_dir: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
                for f in &unanalyzed {
                    let dir = match f.rfind('/') {
                        Some(i) => &f[..i],
                        None => ".",
                    };
                    by_dir.entry(dir).or_default().push(f);
                }
                for (dir, files) in &by_dir {
                    println!("  {} ({} files)", dir, files.len());
                    for f in files {
                        println!("    {}", f);
                    }
                }
            }

            if show_stale && !stale.is_empty() {
                println!();
                println!("Stale files (content changed since analysis):");
                for f in &stale {
                    println!("  {}", f);
                }
            }

            if show_stale && !deleted.is_empty() {
                println!();
                println!("Deleted files (in manifest but missing from disk):");
                for f in &deleted {
                    println!("  {}", f);
                }
            }

            if show_gaps_detail {
                let entity_ids: Vec<&str> = ctx.graph.nodes().into_iter()
                    .map(|n| n.id.raw.as_str())
                    .collect();

                let ext_names: Vec<String> = ctx.manifests.iter().map(|m| m.name.clone()).collect();
                let runtime = specforge_emitter::builtins::runtime_for_extensions(&ext_names);
                let (all_items, scanners_used) = scanner_dispatch::scan_source_files(
                    &runtime,
                    &ctx.manifests,
                    path,
                    &source_files,
                );

                let report = inference::compute_gap_report(all_items, &entity_ids, scanners_used);

                let scanner_label = if report.scanners_used.is_empty() {
                    "no scanners".to_string()
                } else {
                    report.scanners_used.join(", ")
                };
                println!();
                println!("Gap Analysis (via {})", scanner_label);
                println!("  Public items: {}", report.total_pub_items);
                println!("  Covered:      {}", report.covered_items);
                println!("  Gaps:         {}", report.gaps.len());

                if !report.gaps.is_empty() {
                    println!();
                    let mut by_dir: std::collections::BTreeMap<String, Vec<&inference::SourceItem>> =
                        std::collections::BTreeMap::new();
                    for gap in &report.gaps {
                        let dir = match gap.file.rfind('/') {
                            Some(i) => gap.file[..i].to_string(),
                            None => ".".to_string(),
                        };
                        by_dir.entry(dir).or_default().push(gap);
                    }
                    for (dir, gaps) in &by_dir {
                        println!("  {} ({} uncovered)", dir, gaps.len());
                        for g in gaps {
                            println!("    {}:{} {} ({})", g.file, g.line, g.name, g.item_kind);
                        }
                    }
                }
            }
        }
    }

    0
}
