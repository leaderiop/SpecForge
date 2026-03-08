use crate::{Edge, Graph, Node};
use specforge_common::{Diagnostic, Severity};
use specforge_parser::{FieldValue, SpecFile};
use std::collections::HashSet;

pub fn build_graph(spec_files: &[SpecFile]) -> (Graph, Vec<Diagnostic>) {
    let mut graph = Graph::new();
    let mut diagnostics = Vec::new();

    // Add nodes, detecting duplicates
    let mut entity_ids = HashSet::new();
    for spec_file in spec_files {
        for entity in &spec_file.entities {
            if !entity_ids.insert(entity.id.raw.clone()) {
                diagnostics.push(Diagnostic {
                    code: "E002".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "duplicate entity ID '{}' (first declared in {})",
                        entity.id.raw, entity.span.file
                    ),
                    span: Some(entity.span.clone()),
                    suggestion: None,
                });
                continue;
            }
            let node = Node {
                id: entity.id.clone(),
                kind: entity.kind.clone(),
                title: entity.title.clone(),
                fields: entity.fields.clone(),
                source_span: entity.span.clone(),
            };
            graph.add_node(node);
        }
    }

    // Link references → edges
    for spec_file in spec_files {
        for entity in &spec_file.entities {
            for entry in entity.fields.entries() {
                if let FieldValue::ReferenceList(refs) = &entry.value {
                    for target_id in refs {
                        if entity_ids.contains(target_id) {
                            graph.add_edge(Edge {
                                source: entity.id.raw.clone(),
                                target: target_id.clone(),
                                label: entry.key.clone(),
                            });
                        } else {
                            let suggestion = find_close_match(target_id, &entity_ids);
                            diagnostics.push(Diagnostic {
                                code: "E001".to_string(),
                                severity: Severity::Error,
                                message: format!(
                                    "unresolved reference '{}' in entity '{}'",
                                    target_id, entity.id.raw
                                ),
                                span: Some(entity.span.clone()),
                                suggestion: suggestion
                                    .map(|s| format!("did you mean '{}'?", s)),
                            });
                        }
                    }
                }
            }
        }
    }

    (graph, diagnostics)
}

fn find_close_match<'a>(target: &str, candidates: &'a HashSet<String>) -> Option<&'a str> {
    candidates
        .iter()
        .filter_map(|c| {
            let score = strsim::jaro_winkler(target, c);
            if score > 0.85 { Some((c.as_str(), score)) } else { None }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(s, _)| s)
}
