use std::collections::HashMap;
use std::path::Path;

use specforge_common::SourceItem;
use specforge_registry::ManifestV2;
use specforge_wasm::builtin::BuiltinRuntime;
use specforge_wasm::protocol::{ScanRequest, ScanResponse};
use specforge_wasm::runtime::WasmRuntime;

struct ScannerEntry {
    extension_name: String,
    scan_export: String,
}

pub fn scan_source_files(
    runtime: &BuiltinRuntime,
    manifests: &[ManifestV2],
    project_root: &Path,
    source_files: &[String],
) -> (Vec<SourceItem>, Vec<String>) {
    let mut ext_lookup: HashMap<String, ScannerEntry> = HashMap::new();
    for manifest in manifests {
        for ac in &manifest.analyzer_contributions {
            for ext in &ac.file_extensions {
                let normalized = if ext.starts_with('.') {
                    ext.clone()
                } else {
                    format!(".{}", ext)
                };
                ext_lookup.entry(normalized).or_insert_with(|| ScannerEntry {
                    extension_name: manifest.name.clone(),
                    scan_export: ac.scan_export.clone(),
                });
            }
        }
    }

    let mut all_items = Vec::new();
    let mut scanners_used = Vec::new();

    for file_path in source_files {
        let file_ext = match file_path.rfind('.') {
            Some(i) => &file_path[i..],
            None => continue,
        };

        let entry = match ext_lookup.get(file_ext) {
            Some(e) => e,
            None => continue,
        };

        let abs_path = project_root.join(file_path);
        let content = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let req = ScanRequest {
            file_path: file_path.clone(),
            content,
        };
        let input = match serde_json::to_vec(&req) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let result = runtime.call_export(&entry.extension_name, &entry.scan_export, &input);
        if let specforge_wasm::runtime::WasmCallResult::Ok(output) = result {
            if let Ok(resp) = serde_json::from_slice::<ScanResponse>(&output) {
                for item in resp.items {
                    all_items.push(SourceItem {
                        name: item.name,
                        item_kind: item.item_kind,
                        file: file_path.clone(),
                        line: item.line,
                        scanner: Some(entry.extension_name.clone()),
                    });
                }
                if !scanners_used.contains(&entry.extension_name) {
                    scanners_used.push(entry.extension_name.clone());
                }
            }
        }
    }

    (all_items, scanners_used)
}
