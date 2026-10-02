use serde_json::{Value, json};

use specforge_common::AnalyzerConfig;
use specforge_common::inference;
use specforge_common::inference::discovery::SourceDiscoveryConfig;
use specforge_emitter::scanner_dispatch;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, _args: crate::args::NoArgs) -> ToolOutcome {
    let project_root = match &state.project_root {
        Some(p) => p.clone(),
        None => {
            return ToolOutcome::ok(json!({
                "total_pub_items": 0,
                "covered_items": 0,
                "gaps": [],
                "approximate": false,
                "message": "No project root available"
            }));
        }
    };

    let manifest = match inference::load_inference_manifest(&project_root) {
        Ok(m) => m,
        Err(e) => return super::manifest_error(e),
    };

    let analyzer_configs: Vec<AnalyzerConfig> = state
        .registries()
        .manifests
        .iter()
        .flat_map(|m| m.analyzer_contributions.iter())
        .map(|ac| AnalyzerConfig {
            language: ac.language.clone(),
            file_extensions: ac.file_extensions.clone(),
            excluded_dirs: ac.excluded_dirs.clone(),
        })
        .collect();
    let discovery_config = SourceDiscoveryConfig::from_analyzer_configs(&analyzer_configs);
    let source_files =
        inference::discover_source_files(&project_root, &manifest.source_roots, &discovery_config);

    let runtime = state.wasm_runtime(&project_root);
    let (all_items, scanners_used) = scanner_dispatch::scan_source_files(
        runtime.as_ref(),
        &state.registries().manifests,
        &project_root,
        &source_files,
    );

    let entity_ids: Vec<&str> = state
        .graph()
        .nodes()
        .into_iter()
        .map(|n| n.id.raw.as_str())
        .collect();

    let report = inference::compute_gap_report(all_items, &entity_ids, scanners_used);

    let mut by_dir: std::collections::BTreeMap<String, Vec<&inference::SourceItem>> =
        std::collections::BTreeMap::new();
    for gap in &report.gaps {
        let dir = match gap.file.rfind('/') {
            Some(i) => gap.file[..i].to_string(),
            None => ".".to_string(),
        };
        by_dir.entry(dir).or_default().push(gap);
    }

    let dir_breakdown: Vec<Value> = by_dir
        .iter()
        .map(|(dir, gaps)| {
            json!({
                "directory": dir,
                "count": gaps.len(),
                "items": gaps.iter().map(|g| json!({
                    "name": g.name,
                    "item_kind": g.item_kind,
                    "file": g.file,
                    "line": g.line,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let result = json!({
        "total_pub_items": report.total_pub_items,
        "covered_items": report.covered_items,
        "gap_count": report.gaps.len(),
        "approximate": report.approximate,
        "scanners_used": report.scanners_used,
        "by_directory": dir_breakdown,
    });

    ToolOutcome::ok(result)
}
