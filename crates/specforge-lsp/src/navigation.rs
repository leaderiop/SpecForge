use specforge_common::{SourceSpan, Sym};
use specforge_graph::Graph;
use std::path::Path;

/// Returns the declaration location of the entity with the given ID.
pub fn go_to_definition(graph: &Graph, entity_id: &str) -> Option<SourceSpan> {
    graph.node(entity_id).map(|n| n.source_span.clone())
}

/// Returns the file location for a `use` import path.
/// `spec_root` is the root directory for resolving imports.
pub fn goto_import_definition(import_path: &str, spec_root: &str) -> Option<SourceSpan> {
    let file_path = format!("{}/{}.spec", spec_root, import_path);
    if Path::new(&file_path).exists() {
        Some(SourceSpan {
            file: Sym::new(&file_path),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        })
    } else {
        None
    }
}

/// Returns all locations where the entity is referenced (including its declaration).
pub fn find_all_references(graph: &Graph, entity_id: &str) -> Vec<SourceSpan> {
    let mut locations = Vec::new();

    // Include declaration site
    if let Some(node) = graph.node(entity_id) {
        locations.push(node.source_span.clone());
    }

    // Include all nodes that reference this entity via edges
    for edge in graph.edges() {
        if edge.target == entity_id {
            if let Some(source_node) = graph.node(edge.source.as_str()) {
                locations.push(source_node.source_span.clone());
            }
        } else if edge.source == entity_id
            && let Some(target_node) = graph.node(edge.target.as_str())
        {
            locations.push(target_node.source_span.clone());
        }
    }

    locations
}
