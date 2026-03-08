use crate::{Edge, Graph, Node};
use specforge_common::{find_close_match, Diagnostic, Severity};
use specforge_parser::{FieldValue, SpecFile};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct GraphConfig {
    /// Provider schemes that are installed (e.g., "gh", "jira").
    /// Refs with schemes not in this set emit I005.
    pub known_provider_schemes: HashSet<String>,
    /// Entity keywords registered by installed extensions (e.g., "behavior", "feature").
    /// Keywords in this set are considered valid and skip I004 checks.
    pub installed_keywords: HashSet<String>,
    /// Mapping of entity keywords to the extension that provides them (full catalog).
    /// Keywords not in `installed_keywords` but present here emit I004.
    pub known_extension_keywords: HashMap<String, String>,
}

pub fn build_graph(spec_files: &[SpecFile]) -> (Graph, Vec<Diagnostic>) {
    build_graph_with_config(spec_files, &GraphConfig::default())
}

pub fn build_graph_with_config(spec_files: &[SpecFile], config: &GraphConfig) -> (Graph, Vec<Diagnostic>) {
    let mut graph = Graph::new();
    let mut diagnostics = Vec::new();

    // Add nodes, detecting duplicates (same kind + same ID = duplicate)
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut entity_ids = HashSet::new();
    for spec_file in spec_files {
        for entity in &spec_file.entities {
            let key = (entity.kind.raw.clone(), entity.id.raw.clone());
            if !seen.insert(key) {
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
            entity_ids.insert(entity.id.raw.clone());
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

    // Check for unknown keywords that match known extensions (I004)
    if !config.known_extension_keywords.is_empty() {
        for spec_file in spec_files {
            for entity in &spec_file.entities {
                let keyword = &entity.kind.raw;
                // Skip structural kinds that are always valid
                if keyword == "ref" || keyword == "spec" {
                    continue;
                }
                if config.installed_keywords.contains(keyword) {
                    continue;
                }
                if let Some(extension) = config.known_extension_keywords.get(keyword) {
                    diagnostics.push(Diagnostic {
                        code: "I004".to_string(),
                        severity: Severity::Info,
                        message: format!(
                            "keyword '{}' is provided by extension '{}' which is not installed",
                            keyword, extension
                        ),
                        span: Some(entity.span.clone()),
                        suggestion: Some(format!(
                            "install it with: specforge add {}", extension
                        )),
                    });
                }
            }
        }
    }

    // Check ref nodes for unknown provider schemes (I005)
    if !config.known_provider_schemes.is_empty() {
        for spec_file in spec_files {
            for entity in &spec_file.entities {
                if entity.kind.raw == "ref"
                    && let Some(FieldValue::String(scheme)) = entity.fields.get("scheme")
                    && !config.known_provider_schemes.contains(scheme)
                {
                    diagnostics.push(Diagnostic {
                        code: "I005".to_string(),
                        severity: Severity::Info,
                        message: format!(
                            "unrecognized ref scheme '{}' in '{}' — no provider installed for this scheme",
                            scheme, entity.id.raw
                        ),
                        span: Some(entity.span.clone()),
                        suggestion: None,
                    });
                }
            }
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
                            let suggestion = find_close_match(
                                target_id,
                                entity_ids.iter().map(|s| s.as_str()),
                            );
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

