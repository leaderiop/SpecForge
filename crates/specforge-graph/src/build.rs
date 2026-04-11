use crate::{Edge, Graph, Node};
use specforge_common::{Sym, find_close_match, Diagnostic};
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

#[must_use = "diagnostics should be checked for errors"]
pub fn build_graph(spec_files: &[SpecFile]) -> (Graph, Vec<Diagnostic>) {
    build_graph_with_config(spec_files, &GraphConfig::default())
}

#[must_use = "diagnostics should be checked for errors"]
pub fn build_graph_with_config(spec_files: &[SpecFile], config: &GraphConfig) -> (Graph, Vec<Diagnostic>) {
    let mut graph = Graph::new();
    let mut diagnostics = Vec::new();

    // Add nodes, detecting duplicates (same kind + same ID = duplicate)
    let mut seen: HashSet<(Sym, Sym)> = HashSet::new();
    let mut entity_ids: HashSet<Sym> = HashSet::new();
    // Track entity ID to first-seen kind for cross-kind collision detection (W060)
    let mut id_to_kind: HashMap<Sym, Sym> = HashMap::new();
    for spec_file in spec_files {
        for entity in &spec_file.entities {
            let key = (entity.kind.raw, entity.id.raw);
            if !seen.insert(key) {
                diagnostics.push(
                    Diagnostic::error(
                        "E002",
                        format!(
                            "duplicate entity ID '{}' (first declared in {})",
                            entity.id.raw, entity.span.file
                        ),
                    )
                    .with_span(entity.span.clone()),
                );
                continue;
            }

            // Warn when the same ID is used by multiple entity kinds (W060).
            // The graph stores nodes in a HashMap keyed by raw ID alone, so
            // same-ID-different-kind entities will overwrite each other.
            if let Some(&first_kind) = id_to_kind.get(&entity.id.raw) {
                if first_kind != entity.kind.raw {
                    diagnostics.push(
                        Diagnostic::warning(
                            "W060",
                            format!(
                                "entity ID '{}' is used by kind '{}' and kind '{}'; only one will be retained in the graph",
                                entity.id.raw, first_kind, entity.kind.raw
                            ),
                        )
                        .with_span(entity.span.clone()),
                    );
                }
            } else {
                id_to_kind.insert(entity.id.raw, entity.kind.raw);
            }

            entity_ids.insert(entity.id.raw);
            let node = Node {
                id: entity.id,
                kind: entity.kind,
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
                let keyword = entity.kind.raw.as_str();
                // Skip structural kinds that are always valid
                if keyword == "ref" || keyword == "spec" {
                    continue;
                }
                if config.installed_keywords.contains(keyword) {
                    continue;
                }
                if let Some(extension) = config.known_extension_keywords.get(keyword) {
                    diagnostics.push(
                        Diagnostic::info(
                            "I004",
                            format!(
                                "keyword '{}' is provided by extension '{}' which is not installed",
                                keyword, extension
                            ),
                        )
                        .with_span(entity.span.clone())
                        .with_suggestion(format!(
                            "install it with: specforge add {}", extension
                        )),
                    );
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
                    diagnostics.push(
                        Diagnostic::info(
                            "I005",
                            format!(
                                "unrecognized ref scheme '{}' in '{}' — no provider installed for this scheme",
                                scheme, entity.id.raw
                            ),
                        )
                        .with_span(entity.span.clone()),
                    );
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
                        let target_sym = Sym::new(target_id);
                        if entity_ids.contains(&target_sym) {
                            graph.add_edge(Edge {
                                source: entity.id.raw,
                                target: target_sym,
                                label: entry.key,
                            });
                        } else {
                            let suggestion = find_close_match(
                                target_id,
                                entity_ids.iter().map(|s| s.as_str()),
                            );
                            let mut diag = Diagnostic::error(
                                "E001",
                                format!(
                                    "unresolved reference '{}' in entity '{}'",
                                    target_id, entity.id.raw
                                ),
                            )
                            .with_span(entity.span.clone());
                            if let Some(s) = suggestion {
                                diag = diag.with_suggestion(format!("did you mean '{}'?", s));
                            }
                            diagnostics.push(diag);
                        }
                    }
                }
            }
        }
    }

    // Detect reference cycles and emit W061
    let cycles = graph.detect_cycles();
    for cycle in &cycles {
        let path: Vec<String> = cycle.iter().map(|s| s.to_string()).collect();
        diagnostics.push(
            Diagnostic::warning(
                "W061",
                format!("reference cycle detected: {}", path.join(" -> ")),
            ),
        );
    }

    (graph, diagnostics)
}
