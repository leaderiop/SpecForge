use crate::ValidatorConfig;
use specforge_common::{find_close_match, Diagnostic};
use specforge_graph::Graph;
use specforge_parser::FieldValue;
use std::path::Path;

pub fn validate_file_references(
    graph: &Graph,
    config: &ValidatorConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if config.file_reference_fields.is_empty() {
        return;
    }

    for node in graph.nodes() {
        for field_name in &config.file_reference_fields {
            if let Some(FieldValue::StringList(paths)) = node.fields.get(field_name) {
                for path in paths {
                    let full_path = config.spec_root.join(path);
                    if !full_path.exists() {
                        let suggestion = suggest_similar_file(path, &config.spec_root);
                        let mut diag = Diagnostic::error(
                            "E016",
                            format!(
                                "file reference '{}' in entity '{}' does not exist",
                                path, node.id.raw
                            ),
                        )
                        .with_span(node.source_span.clone());
                        if let Some(s) = suggestion {
                            diag = diag.with_suggestion(s);
                        }
                        diagnostics.push(diag);
                    }
                }
            }
        }
    }
}

fn suggest_similar_file(path: &str, spec_root: &Path) -> Option<String> {
    let full_path = spec_root.join(path);
    let parent = full_path.parent()?;
    if !parent.is_dir() {
        return None;
    }

    let file_name = full_path.file_name()?.to_str()?;
    let siblings: Vec<String> = std::fs::read_dir(parent)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let match_name = find_close_match(file_name, siblings.iter().map(|s| s.as_str()))?;

    // Reconstruct the relative path with the suggested filename
    let path_obj = Path::new(path);
    let suggested = if let Some(dir) = path_obj.parent().filter(|p| !p.as_os_str().is_empty()) {
        format!("{}/{}", dir.display(), match_name)
    } else {
        match_name.to_string()
    };

    Some(format!("did you mean '{}'?", suggested))
}
