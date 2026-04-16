use crate::{Graph, Node};
use specforge_common::{Diagnostic, Sym};
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
    /// Bidirectional edge pairs from extensions. Each pair (forward_label, reverse_label)
    /// represents a complementary relationship that should not be flagged as a cycle.
    /// Example: `("invariants", "enforced_by")` means an invariants/enforced_by 2-hop
    /// cycle is a known bidirectional relationship, not a real circular dependency.
    pub bidirectional_pairs: Vec<(String, String)>,
}

#[must_use = "diagnostics should be checked for errors"]
pub fn build_graph(spec_files: &[SpecFile]) -> (Graph, Vec<Diagnostic>) {
    build_graph_with_config(spec_files, &GraphConfig::default())
}

#[must_use = "diagnostics should be checked for errors"]
pub fn build_graph_with_config(spec_files: &[SpecFile], config: &GraphConfig) -> (Graph, Vec<Diagnostic>) {
    let mut graph = Graph::with_bidirectional_pairs(config.bidirectional_pairs.clone());
    let mut diagnostics = Vec::new();

    // Surface parse errors as diagnostics so CLI/MCP consumers see them
    for spec_file in spec_files {
        for error in &spec_file.errors {
            diagnostics.push(Diagnostic::from(error));
        }
    }

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
                    .with_span(entity.span.clone())
                    .with_suggestion("rename one of the entities to avoid the collision"),
                );
                continue;
            }

            // Warn when the same ID is used by multiple entity kinds (W060).
            // First-writer-wins: the first entity with a given raw ID is retained,
            // subsequent entities with the same ID but different kind are skipped.
            if let Some(&first_kind) = id_to_kind.get(&entity.id.raw) {
                if first_kind != entity.kind.raw {
                    diagnostics.push(
                        Diagnostic::warning(
                            "W060",
                            format!(
                                "entity ID '{}' is used by kind '{}' and kind '{}'; first declaration (kind '{}') is retained",
                                entity.id.raw, first_kind, entity.kind.raw, first_kind
                            ),
                        )
                        .with_span(entity.span.clone())
                        .with_suggestion("use distinct IDs for entities of different kinds"),
                    );
                    continue;
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
                                "unrecognized ref scheme '{}' in '{}' --- no provider installed for this scheme",
                                scheme, entity.id.raw
                            ),
                        )
                        .with_span(entity.span.clone()),
                    );
                }
            }
        }
    }

    // Link references -> edges (shared with LSP via Graph::resolve_references)
    let ref_diags = graph.resolve_references();
    diagnostics.extend(ref_diags);

    // Detect reference cycles and emit W061
    let cycles = graph.detect_cycles();
    for cycle in &cycles {
        let path: Vec<String> = cycle.iter().map(|s| s.to_string()).collect();
        diagnostics.push(
            Diagnostic::warning(
                "W061",
                format!("reference cycle detected: {}", path.join(" -> ")),
            )
            .with_suggestion("break the cycle by removing or inverting one reference"),
        );
    }

    (graph, diagnostics)
}
