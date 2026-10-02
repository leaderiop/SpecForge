use specforge_common::{SourceSpan, Sym};
use specforge_graph::Graph;
use specforge_resolver::{ResolveConfig, resolve_import};
use std::path::Path;

/// Returns the declaration location of the entity with the given ID.
pub fn go_to_definition(graph: &Graph, entity_id: &str) -> Option<SourceSpan> {
    graph.node(entity_id).map(|n| n.source_span.clone())
}

/// The file a `use` import path in `importing_file` (relative to
/// `spec_root`) names, resolved as the compile resolves it (relative,
/// `@alias`, bare, `index.spec`, never above the spec root): its first
/// line, keyed relative to the spec root. `None` when it names no file.
pub fn goto_import_definition(
    import_path: &str,
    importing_file: &str,
    spec_root: &Path,
    config: &ResolveConfig,
) -> Option<SourceSpan> {
    let target = resolve_import(spec_root, importing_file, import_path, config)?;
    Some(SourceSpan {
        file: Sym::new(&target),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    })
}

/// Returns all locations where the entity is referenced (including its
/// declaration): the entities on either end of its edges.
pub fn find_all_references(graph: &Graph, entity_id: &str) -> Vec<SourceSpan> {
    let declaration = graph.node(entity_id).map(|n| n.source_span.clone());
    let referencing = graph.edges_to(entity_id).into_iter().map(|e| e.source);
    let referenced = graph.edges_from(entity_id).into_iter().map(|e| e.target);
    declaration
        .into_iter()
        .chain(
            referencing
                .chain(referenced)
                .filter_map(|id| graph.node(id.as_str()))
                .map(|n| n.source_span.clone()),
        )
        .collect()
}
