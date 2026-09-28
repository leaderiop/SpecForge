use crate::ResolvedProject;
use specforge_common::{Diagnostic, Severity, Sym, find_close_match};
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
                // Same-file same-(kind, id) duplicates are caught by
                // build.rs as E002. Cross-file occurrences warn here —
                // same kind is a duplicate definition (W063), a *different*
                // kind under the same ID is an ambiguous identity and is
                // called out as such (C3-07: previously silent).
                if first_file != file.path.as_str() {
                    if first_kind == entity.kind.raw {
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
                    } else {
                        diagnostics.push(Diagnostic {
                            code: "W063".to_string(),
                            severity: Severity::Warning,
                            message: format!(
                                "entity ID '{}' is declared as '{}' in '{}' but as '{}' in '{}' — one ID, two kinds",
                                entity.id.raw, first_kind, first_file, entity.kind.raw, file.path
                            ),
                            span: Some(entity.span.clone()),
                            suggestion: Some(
                                "entity IDs share one flat namespace regardless of kind; rename one occurrence"
                                    .to_string(),
                            ),
                        });
                    }
                }
            } else {
                id_first_seen.insert(entity.id.raw, (file.path.as_str(), entity.kind.raw));
            }
            entity_ids.insert(entity.id.raw, ());
        }
    }

    let all_ids: Vec<&str> = entity_ids.keys().map(|s| s.as_str()).collect();

    // C3-06: per-file visibility. A file may reference entities it declares,
    // plus entities exported by the files it imports. The global index remains
    // as a fallback, but a reference that only resolves globally now emits
    // W099 (advisory — the edge is still created) instead of resolving
    // silently. Deliberately permissive: an aliased selective import makes
    // both the alias and the whole target's exports visible; tightening that
    // is a follow-up once warning data exists.
    let mut visible_by_file: HashMap<&str, std::collections::HashSet<String>> = HashMap::new();
    for file in &project.files {
        let scope = project.file_scopes.get(file.path.as_str());
        let mut visible: std::collections::HashSet<String> = scope
            .map(|s| s.declared.iter().cloned().collect())
            .unwrap_or_default();
        if project.file_scopes.contains_key(file.path.as_str()) {
            for target in &file.import_targets {
                if let Some(target_scope) = project.file_scopes.get(target) {
                    visible.extend(target_scope.exported.iter().cloned());
                }
            }
        }
        // Selective imports with aliases: the alias is the referenceable name.
        for import in &file.spec_file.imports {
            if let Some(bindings) = &import.bindings {
                for binding in bindings {
                    if let Some(alias) = &binding.alias {
                        visible.insert(alias.clone());
                    }
                }
            }
        }
        visible_by_file.insert(file.path.as_str(), visible);
    }

    // Walk all entities, find reference lists, and create edges
    for file in &project.files {
        let visible = visible_by_file.get(file.path.as_str());
        for entity in &file.spec_file.entities {
            for entry in entity.fields.entries() {
                if let FieldValue::ReferenceList(refs) = &entry.value {
                    for target_ref in refs {
                        let target_id = target_ref.as_str();
                        let target_sym = Sym::new(target_id);
                        if entity_ids.contains_key(&target_sym) {
                            // Advisory visibility check (C3-06): known but not
                            // imported means the file relies on global scope.
                            let imported = visible.map(|v| v.contains(target_id)).unwrap_or(true);
                            if !imported {
                                diagnostics.push(Diagnostic {
                                    code: "W099".to_string(),
                                    severity: Severity::Warning,
                                    message: format!(
                                        "reference '{}' in entity '{}' resolves outside the file's import graph",
                                        target_id, entity.id.raw
                                    ),
                                    span: Some(entity.span.clone()),
                                    suggestion: Some(format!(
                                        "add `use \"...{}...\"` to make the dependency explicit",
                                        target_id
                                    )),
                                });
                            }
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
                                span: Some(target_ref.span.clone()),
                                suggestion: suggestion.map(|s| format!("did you mean '{}'?", s)),
                            });
                        }
                    }
                }
            }
        }
    }

    (edges, diagnostics)
}
