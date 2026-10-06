use specforge_common::{Diagnostic, codes};
use specforge_graph::{FieldValue, Graph};

/// Resolver integrity: a reference to an entity that exists must have become
/// a graph edge (source, target, field). One that didn't is a resolver bug,
/// E060. A reference to a missing entity is the linker's E003, not ours.
pub fn detect_dangling_references(graph: &Graph, diagnostics: &mut Vec<Diagnostic>) {
    for node in graph.nodes() {
        let source = node.id.raw.as_str();
        let edges = graph.edges_from(source);
        for entry in node.fields.entries() {
            let FieldValue::ReferenceList(refs) = &entry.value else {
                continue;
            };
            for target in refs {
                if graph.node(&target.id).is_none() {
                    continue;
                }
                let linked = edges
                    .iter()
                    .any(|e| e.target == target.id.as_str() && e.label == entry.key);
                if !linked {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::E060,
                            format!(
                                "reference '{}' in field '{}' of '{}' resolved but has no graph edge",
                                target.id, entry.key, source
                            ),
                        )
                        .with_span(target.span.clone())
                        .with_suggestion(
                            "this is a SpecForge bug, not a spec error: please report it",
                        ),
                    );
                }
            }
        }
    }
}
