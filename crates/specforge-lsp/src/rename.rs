use specforge_common::SourceSpan;
use specforge_graph::Graph;

/// A text edit for a rename operation.
#[derive(Debug, Clone)]
pub struct RenameEdit {
    pub file: String,
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub new_text: String,
}

/// Validates that the entity can be renamed and returns its token range.
pub fn prepare_rename(graph: &Graph, entity_id: &str) -> Option<SourceSpan> {
    graph.node(entity_id).map(|n| n.source_span.clone())
}

/// Compute all text edits needed to rename an entity across the graph.
/// Returns None if `old_id` doesn't exist or `new_id` already exists (duplicate).
pub fn compute_rename_edits(
    graph: &Graph,
    old_id: &str,
    new_id: &str,
) -> Option<Vec<RenameEdit>> {
    // Reject if old doesn't exist
    graph.node(old_id)?;

    // Reject if new already exists (would create duplicate)
    if graph.node(new_id).is_some() {
        return None;
    }

    let mut edits = Vec::new();

    // Edit at the declaration site
    let decl = &graph.node(old_id).unwrap().source_span;
    edits.push(RenameEdit {
        file: decl.file.to_string(),
        line: decl.start_line,
        start_col: decl.start_col,
        end_col: decl.end_col,
        new_text: new_id.to_string(),
    });

    // Edit at every reference site (nodes connected by edges)
    for edge in graph.edges() {
        if edge.target == old_id
            && let Some(source_node) = graph.node(edge.source.as_str())
        {
            edits.push(RenameEdit {
                file: source_node.source_span.file.to_string(),
                line: source_node.source_span.start_line,
                start_col: source_node.source_span.start_col,
                end_col: source_node.source_span.end_col,
                new_text: new_id.to_string(),
            });
        }
    }

    Some(edits)
}
