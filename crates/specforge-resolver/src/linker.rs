use crate::ResolvedProject;
use specforge_common::{find_close_match, Diagnostic, Severity, Sym};
use specforge_parser::FieldValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct PendingEdge {
    pub source: Sym,
    pub target: Sym,
    pub label: Sym,
}

pub fn link_references(project: &ResolvedProject) -> (Vec<PendingEdge>, Vec<Diagnostic>) {
    let mut edges = Vec::new();
    let mut diagnostics = Vec::new();

    // Build entity index: id -> true (exists)
    let mut entity_ids: HashMap<Sym, ()> = HashMap::new();
    for file in &project.files {
        for entity in &file.spec_file.entities {
            entity_ids.insert(entity.id.raw, ());
        }
    }

    let all_ids: Vec<&str> = entity_ids.keys().map(|s| s.as_str()).collect();

    // Walk all entities, find reference lists, and create edges
    for file in &project.files {
        for entity in &file.spec_file.entities {
            for entry in entity.fields.entries() {
                if let FieldValue::ReferenceList(refs) = &entry.value {
                    for target_id in refs {
                        let target_sym = Sym::new(target_id);
                        if entity_ids.contains_key(&target_sym) {
                            edges.push(PendingEdge {
                                source: entity.id.raw,
                                target: target_sym,
                                label: entry.key,
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

