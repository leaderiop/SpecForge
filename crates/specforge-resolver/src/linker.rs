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

    // Build entity index: id -> (file, kind) for the first occurrence.
    // Detect cross-file duplicate entity IDs and emit a diagnostic.
    let mut entity_ids: HashMap<Sym, ()> = HashMap::new();
    let mut id_first_seen: HashMap<Sym, (&str, Sym)> = HashMap::new();
    for file in &project.files {
        for entity in &file.spec_file.entities {
            if let Some(&(first_file, first_kind)) = id_first_seen.get(&entity.id.raw) {
                // Same (kind, id) duplicates are caught by build.rs as E002.
                // Here we detect cross-file same-ID occurrences and warn
                // only when the first occurrence was in a *different* file,
                // to avoid double-reporting with the graph builder's E002.
                if first_file != file.path.as_str() && first_kind == entity.kind.raw {
                    diagnostics.push(Diagnostic {
                        code: "W063".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "entity ID '{}' defined in '{}' was already defined in '{}'",
                            entity.id.raw, file.path, first_file
                        ),
                        span: Some(entity.span.clone()),
                        suggestion: Some(
                            "use unique entity IDs across files, or use imports to share entities"
                                .to_string(),
                        ),
                    });
                }
            } else {
                id_first_seen.insert(entity.id.raw, (file.path.as_str(), entity.kind.raw));
            }
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
                                code: "E003".to_string(),
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
