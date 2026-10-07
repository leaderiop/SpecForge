mod dangling;

pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_graph::Graph;

/// The linker's integrity check: a reference to an existing entity must
/// have become a graph edge (E060).
pub fn validate(graph: &Graph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    dangling::detect_dangling_references(graph, &mut diagnostics);
    diagnostics
}
