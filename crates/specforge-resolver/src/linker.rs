use crate::ResolvedProject;
use specforge_common::{find_close_match, Diagnostic, Severity};
use specforge_parser::FieldValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PendingEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

pub fn link_references(project: &ResolvedProject) -> (Vec<PendingEdge>, Vec<Diagnostic>) {
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();

    // Build entity index: id -> true (exists)
    let mut entity_ids: HashMap<String, ()> = HashMap::new();
    for file in &project.files {
        for entity in &file.spec_file.entities {
            entity_ids.insert(entity.id.raw.clone(), ());
        }
    }

    let all_ids: Vec<&str> = entity_ids.keys().map(|s| s.as_str()).collect();

    // Walk all entities, find reference lists, and create edges
    for file in &project.files {
        for entity in &file.spec_file.entities {
            for entry in entity.fields.entries() {
                if let FieldValue::ReferenceList(refs) = &entry.value {
                    for target_id in refs {
                        if entity_ids.contains_key(target_id) {
                            edges.push(PendingEdge {
                                source: entity.id.raw.clone(),
                                target: target_id.clone(),
                                label: entry.key.clone(),
                            });
                        } else {
                            let suggestion = find_close_match(target_id, all_ids.iter().copied());
                            diagnostics.push(Diagnostic {
                                code: "E001".to_string(),
                                severity: Severity::Error,
                                message: format!(
                                    "unresolved reference '{}' in entity '{}'",
                                    target_id, entity.id.raw
                                ),
                                span: Some(entity.span.clone()),
                                suggestion: suggestion.map(|s| {
                                    format!("did you mean '{}'?", s)
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    (edges, diagnostics)
}

