use crate::trace::compute_trace;
use crate::types::{GeneratedFile, RenderOptions};
use specforge_common::{CompilerConfig, EntityKind, FieldValue};
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;
use std::fmt::Write;

/// Return the markdown filename for a given entity kind.
fn plural_filename(kind: EntityKind) -> String {
    match kind {
        EntityKind::Spec => "spec.md".to_string(),
        EntityKind::Glossary => "glossary.md".to_string(),
        EntityKind::Library => "libraries.md".to_string(),
        EntityKind::Capability => "capabilities.md".to_string(),
        EntityKind::Deliverable => "deliverables.md".to_string(),
        EntityKind::Invariant => "invariants.md".to_string(),
        EntityKind::Behavior => "behaviors.md".to_string(),
        EntityKind::Feature => "features.md".to_string(),
        EntityKind::Event => "events.md".to_string(),
        EntityKind::TypeDef => "types.md".to_string(),
        EntityKind::Port => "ports.md".to_string(),
        EntityKind::Ref => "refs.md".to_string(),
        EntityKind::Roadmap => "roadmaps.md".to_string(),
        EntityKind::Decision => "decisions.md".to_string(),
        EntityKind::Constraint => "constraints.md".to_string(),
        EntityKind::FailureMode => "failure_modes.md".to_string(),
    }
}

/// Return a display title for a group of entities of a kind.
fn plural_title(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Spec => "Spec",
        EntityKind::Glossary => "Glossary",
        EntityKind::Library => "Libraries",
        EntityKind::Capability => "Capabilities",
        EntityKind::Deliverable => "Deliverables",
        EntityKind::Invariant => "Invariants",
        EntityKind::Behavior => "Behaviors",
        EntityKind::Feature => "Features",
        EntityKind::Event => "Events",
        EntityKind::TypeDef => "Types",
        EntityKind::Port => "Ports",
        EntityKind::Ref => "Refs",
        EntityKind::Roadmap => "Roadmaps",
        EntityKind::Decision => "Decisions",
        EntityKind::Constraint => "Constraints",
        EntityKind::FailureMode => "Failure Modes",
    }
}

/// Extract the primary description field for a given entity kind.
fn description_field_key(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Behavior => "contract",
        EntityKind::Invariant => "guarantee",
        EntityKind::Feature => "problem",
        EntityKind::Capability => "flow",
        EntityKind::Constraint => "statement",
        EntityKind::Decision => "context",
        EntityKind::FailureMode => "description",
        _ => "description",
    }
}

/// Convert an entity ID to a markdown anchor (lowercase, no special handling needed for IDs).
fn to_anchor(id: &str) -> String {
    id.to_lowercase()
}

/// Render markdown docs — returns one file per entity kind + index.md (BEH-SF-060, 068).
/// Respects options.only for filtering (BEH-SF-069).
pub fn render_markdown(
    graph: &SpecGraph,
    files: &[SpecFile],
    config: &CompilerConfig,
    options: &RenderOptions,
) -> Vec<GeneratedFile> {
    let mut generated = Vec::new();

    // Build a map of entity_id → AstEntity for field access
    let mut entity_fields = std::collections::HashMap::new();
    for file in files {
        for entity in &file.entities {
            entity_fields.insert(entity.id.raw().to_string(), entity);
        }
    }

    // Determine which kinds to render
    let kinds: Vec<EntityKind> = if let Some(only_kind) = options.only {
        vec![only_kind]
    } else {
        EntityKind::ALL.to_vec()
    };

    // Generate one file per populated kind
    for kind in &kinds {
        let mut nodes: Vec<_> = graph
            .nodes_of_kind(*kind)
            .into_iter()
            .collect();
        nodes.sort_by_key(|n| n.id.raw().to_string());

        if nodes.is_empty() {
            continue;
        }

        let mut content = String::new();
        writeln!(content, "# {}\n", plural_title(*kind)).unwrap();

        for node in &nodes {
            let id = node.id.raw();
            let title = node.title.as_deref().unwrap_or("(untitled)");
            writeln!(content, "## {id}: {title}\n").unwrap();
            writeln!(content, "**Kind:** {}", kind.keyword()).unwrap();
            writeln!(content, "**File:** {}\n", node.file).unwrap();

            // Description
            if let Some(ast_entity) = entity_fields.get(id) {
                let desc_key = description_field_key(*kind);
                if let Some(field_value) = ast_entity.fields.get(desc_key) {
                    let section_title = match *kind {
                        EntityKind::Behavior => "Contract",
                        EntityKind::Invariant => "Guarantee",
                        EntityKind::Feature => "Problem",
                        EntityKind::Capability => "Flow",
                        EntityKind::Constraint => "Statement",
                        EntityKind::Decision => "Context",
                        EntityKind::FailureMode => "Description",
                        _ => "Description",
                    };
                    if let FieldValue::String(text) = field_value {
                        writeln!(content, "### {section_title}").unwrap();
                        writeln!(content, "{text}\n").unwrap();
                    }
                }

                // For features, also show solution
                if *kind == EntityKind::Feature {
                    if let Some(FieldValue::String(sol)) = ast_entity.fields.get("solution") {
                        writeln!(content, "### Solution").unwrap();
                        writeln!(content, "{sol}\n").unwrap();
                    }
                }
            }

            // Cross-references via outgoing edges
            let outgoing = graph.outgoing_edges(id);
            if !outgoing.is_empty() {
                writeln!(content, "### References").unwrap();
                let mut refs: Vec<_> = outgoing
                    .iter()
                    .map(|(tgt, edge)| {
                        let tgt_id = tgt.id.raw().to_string();
                        let tgt_kind = tgt.kind;
                        let file = plural_filename(tgt_kind);
                        let anchor = to_anchor(&tgt_id);
                        (
                            plural_title(tgt_kind).to_string(),
                            format!("[{tgt_id}]({file}#{anchor})"),
                            edge.edge_type.label().to_string(),
                        )
                    })
                    .collect();
                refs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

                for (group, link, edge_label) in &refs {
                    writeln!(content, "- {group}: {link} ({edge_label})").unwrap();
                }
                writeln!(content).unwrap();
            }

            // Traceability section
            if let Some(chain) = compute_trace(graph, id) {
                let has_upstream = !chain.upstream.is_empty();
                let has_downstream = !chain.downstream.is_empty();

                if has_upstream || has_downstream {
                    writeln!(content, "### Traceability").unwrap();

                    for link in &chain.upstream {
                        if let Some(from_node) = graph.get_node(&link.from) {
                            let from_kind = from_node.kind;
                            let file = plural_filename(from_kind);
                            let anchor = to_anchor(&link.from);
                            writeln!(
                                content,
                                "- Upstream: [{from}]({file}#{anchor}) ({})",
                                from_kind.keyword(),
                                from = link.from,
                            )
                            .unwrap();
                        }
                    }
                    for link in &chain.downstream {
                        if let Some(to_node) = graph.get_node(&link.to) {
                            let to_kind = to_node.kind;
                            let file = plural_filename(to_kind);
                            let anchor = to_anchor(&link.to);
                            writeln!(
                                content,
                                "- Downstream: [{to}]({file}#{anchor}) ({})",
                                to_kind.keyword(),
                                to = link.to,
                            )
                            .unwrap();
                        }
                    }
                    writeln!(content).unwrap();
                }
            }
        }

        generated.push(GeneratedFile {
            path: plural_filename(*kind),
            content,
        });
    }

    // Generate index.md
    let index = render_index(graph, config, &kinds);
    generated.push(index);

    generated
}

/// Render index.md listing all entities grouped by kind.
fn render_index(graph: &SpecGraph, config: &CompilerConfig, kinds: &[EntityKind]) -> GeneratedFile {
    let mut content = String::new();
    writeln!(content, "# {} Specification\n", config.name).unwrap();
    writeln!(content, "## Entities\n").unwrap();

    for kind in kinds {
        let mut nodes: Vec<_> = graph.nodes_of_kind(*kind).into_iter().collect();
        nodes.sort_by_key(|n| n.id.raw().to_string());

        if nodes.is_empty() {
            continue;
        }

        let file = plural_filename(*kind);
        writeln!(content, "### {} ({})\n", plural_title(*kind), nodes.len()).unwrap();

        for node in &nodes {
            let id = node.id.raw();
            let title = node.title.as_deref().unwrap_or("(untitled)");
            let anchor = to_anchor(id);
            writeln!(content, "- [{id}]({file}#{anchor}) — {title}").unwrap();
        }
        writeln!(content).unwrap();
    }

    GeneratedFile {
        path: "index.md".to_string(),
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{
        CompilerConfig, EdgeType, EntityId, EntityKind, FieldMap, FieldValue, SourceSpan,
    };
    use specforge_graph::{GraphEdge, GraphNode, SpecGraph};
    use specforge_parser::{AstEntity, SpecFile};

    fn test_config() -> CompilerConfig {
        CompilerConfig::core_only("test-project")
    }

    fn make_node(id: &str, kind: EntityKind, title: &str, file: &str) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(title.to_string()),
            file: file.to_string(),
            span: SourceSpan::new(file, 1, 1, 1, 1),
        }
    }

    fn make_entity(id: &str, kind: EntityKind, title: &str, fields: FieldMap) -> AstEntity {
        AstEntity {
            kind,
            id: EntityId::parse(id),
            title: Some(title.to_string()),
            fields,
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    fn make_test_graph_and_files() -> (SpecGraph, Vec<SpecFile>) {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node(
            "data_integrity",
            EntityKind::Invariant,
            "Data Integrity",
            "invariants.spec",
        ));
        graph.add_node(make_node(
            "validate_input",
            EntityKind::Behavior,
            "Validate Input",
            "behaviors.spec",
        ));
        graph.add_node(make_node(
            "input_validation",
            EntityKind::Feature,
            "Input Validation",
            "features.spec",
        ));
        graph.add_edge(
            "validate_input",
            "data_integrity",
            GraphEdge {
                edge_type: EdgeType::References,
                field_name: "invariants".to_string(),
            },
        );
        graph.add_edge(
            "input_validation",
            "validate_input",
            GraphEdge {
                edge_type: EdgeType::Implements,
                field_name: "behaviors".to_string(),
            },
        );

        let mut beh_fields = FieldMap::new();
        beh_fields.insert("contract", FieldValue::String("the system MUST validate all input fields".to_string()));

        let mut inv_fields = FieldMap::new();
        inv_fields.insert("guarantee", FieldValue::String("all data MUST be validated before persistence".to_string()));

        let mut feat_fields = FieldMap::new();
        feat_fields.insert("problem", FieldValue::String("invalid data can corrupt the system".to_string()));
        feat_fields.insert("solution", FieldValue::String("validate all input before processing".to_string()));

        let files = vec![SpecFile {
            path: "test.spec".to_string(),
            imports: vec![],
            entities: vec![
                make_entity("data_integrity", EntityKind::Invariant, "Data Integrity", inv_fields),
                make_entity("validate_input", EntityKind::Behavior, "Validate Input", beh_fields),
                make_entity("input_validation", EntityKind::Feature, "Input Validation", feat_fields),
            ],
            errors: vec![],
        }];

        (graph, files)
    }

    #[test]
    fn markdown_one_file_per_kind() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);

        // Should have one file per populated kind (3) + index = 4
        let file_names: Vec<_> = result.iter().map(|f| f.path.as_str()).collect();
        assert!(file_names.contains(&"invariants.md"));
        assert!(file_names.contains(&"behaviors.md"));
        assert!(file_names.contains(&"features.md"));
        assert!(file_names.contains(&"index.md"));
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn markdown_includes_titles() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);
        let beh_file = result.iter().find(|f| f.path == "behaviors.md").unwrap();
        assert!(beh_file.content.contains("Validate Input"));
    }

    #[test]
    fn markdown_includes_description() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);

        let beh_file = result.iter().find(|f| f.path == "behaviors.md").unwrap();
        assert!(beh_file.content.contains("### Contract"));
        assert!(beh_file.content.contains("the system MUST validate all input fields"));

        let inv_file = result.iter().find(|f| f.path == "invariants.md").unwrap();
        assert!(inv_file.content.contains("### Guarantee"));
    }

    #[test]
    fn markdown_cross_references_as_links() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);
        let beh_file = result.iter().find(|f| f.path == "behaviors.md").unwrap();
        // validate_input references data_integrity
        assert!(beh_file.content.contains("[data_integrity](invariants.md#data_integrity)"));
    }

    #[test]
    fn markdown_traceability_section() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);
        let beh_file = result.iter().find(|f| f.path == "behaviors.md").unwrap();
        assert!(beh_file.content.contains("### Traceability"));
        assert!(beh_file.content.contains("Upstream:"));
        assert!(beh_file.content.contains("Downstream:"));
    }

    #[test]
    fn markdown_only_filter() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions {
            only: Some(EntityKind::Behavior),
        };
        let result = render_markdown(&graph, &files, &test_config(), &options);
        let file_names: Vec<_> = result.iter().map(|f| f.path.as_str()).collect();
        assert!(file_names.contains(&"behaviors.md"));
        assert!(file_names.contains(&"index.md"));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn markdown_empty_graph() {
        let graph = SpecGraph::new();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &[], &test_config(), &options);
        // Only index.md
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "index.md");
    }

    #[test]
    fn markdown_deterministic() {
        let (graph, files) = make_test_graph_and_files();
        let config = test_config();
        let options = RenderOptions::default();
        let a = render_markdown(&graph, &files, &config, &options);
        let b = render_markdown(&graph, &files, &config, &options);
        for (fa, fb) in a.iter().zip(b.iter()) {
            assert_eq!(fa.path, fb.path);
            assert_eq!(fa.content, fb.content);
        }
    }

    #[test]
    fn index_lists_all_entities() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);
        let index = result.iter().find(|f| f.path == "index.md").unwrap();
        assert!(index.content.contains("data_integrity"));
        assert!(index.content.contains("validate_input"));
        assert!(index.content.contains("input_validation"));
    }

    #[test]
    fn markdown_snapshot() {
        let (graph, files) = make_test_graph_and_files();
        let options = RenderOptions::default();
        let result = render_markdown(&graph, &files, &test_config(), &options);
        // Snapshot the index
        let index = result.iter().find(|f| f.path == "index.md").unwrap();
        insta::assert_snapshot!("markdown_index", &index.content);
        // Snapshot behaviors
        let beh = result.iter().find(|f| f.path == "behaviors.md").unwrap();
        insta::assert_snapshot!("markdown_behaviors", &beh.content);
    }
}
