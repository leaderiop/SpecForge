use specforge_common::SourceSpan;
use specforge_graph::Graph;

/// A symbol entry for outline view or workspace search.
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub id: String,
    pub kind: String,
    pub title: Option<String>,
    pub span: SourceSpan,
}

/// Return all entity symbols in a given file (for document outline).
pub fn document_symbols(graph: &Graph, file: &str) -> Vec<SymbolEntry> {
    let mut symbols: Vec<SymbolEntry> = graph
        .nodes_in_file(file)
        .into_iter()
        .map(|n| SymbolEntry {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
            span: n.source_span.clone(),
        })
        .collect();
    symbols.sort_by_key(|s| s.span.start_line);
    symbols
}

/// Search all entities in the workspace by ID prefix or title fragment.
pub fn workspace_symbols(graph: &Graph, query: &str) -> Vec<SymbolEntry> {
    let query_lower = query.to_lowercase();
    let mut symbols: Vec<SymbolEntry> = graph
        .nodes()
        .into_iter()
        .filter(|n| {
            n.id.raw.as_str().to_lowercase().contains(&query_lower)
                || n.title
                    .as_ref()
                    .is_some_and(|t| t.to_lowercase().contains(&query_lower))
        })
        .map(|n| SymbolEntry {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
            span: n.source_span.clone(),
        })
        .collect();
    symbols.sort_by(|a, b| a.id.cmp(&b.id));
    symbols
}
