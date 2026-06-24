use serde_json::{json, Value};

use specforge_common::inference;
use specforge_common::inference::discovery::SourceDiscoveryConfig;
use specforge_common::AnalyzerConfig;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    let project_root = match &state.project_root {
        Some(p) => p.clone(),
        None => {
            return JsonRpcResponse::success(id, json!({
                "content": [{ "type": "text", "text": json!({
                    "summary": { "files_total": 0, "files_analyzed": 0, "entities_produced": 0 },
                    "unanalyzed": [],
                    "stale": [],
                    "deleted": [],
                    "message": "No project root available"
                }).to_string() }]
            }));
        }
    };

    let manifest = match inference::load_inference_manifest(&project_root) {
        Ok(m) => m,
        Err(e) => {
            return JsonRpcResponse::success(id, json!({
                "content": [{ "type": "text", "text": json!({
                    "error": e,
                }).to_string() }]
            }));
        }
    };

    let analyzer_configs: Vec<AnalyzerConfig> = state.manifests.iter()
        .flat_map(|m| m.analyzer_contributions.iter())
        .map(|ac| AnalyzerConfig {
            language: ac.language.clone(),
            file_extensions: ac.file_extensions.clone(),
            excluded_dirs: ac.excluded_dirs.clone(),
        })
        .collect();
    let discovery_config = SourceDiscoveryConfig::from_analyzer_configs(&analyzer_configs);
    let source_files = inference::discover_source_files(&project_root, &manifest.source_roots, &discovery_config);
    let index_map = manifest.source_index_map();

    let unanalyzed: Vec<&str> = source_files
        .iter()
        .filter(|f| !index_map.contains_key(f.as_str()))
        .map(|f| f.as_str())
        .collect();

    let (stale, deleted) = inference::detect_stale_entries(&project_root, &manifest);
    let summary = manifest.compute_summary(source_files.len());

    let result = json!({
        "summary": {
            "files_total": summary.files_total,
            "files_analyzed": summary.files_analyzed,
            "entities_produced": summary.entities_produced,
        },
        "unanalyzed": unanalyzed,
        "stale": stale,
        "deleted": deleted,
    });

    JsonRpcResponse::success(id, json!({
        "content": [{ "type": "text", "text": result.to_string() }]
    }))
}

