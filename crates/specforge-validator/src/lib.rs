mod file_ref;
mod orphan;
mod render;
mod summary;

pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_graph::Graph;
pub use render::render_diagnostics;
pub use summary::{diagnostic_summary, diagnostic_summary_detailed};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ValidatorConfig {
    /// Root directory for resolving file references.
    pub spec_root: PathBuf,
    /// Field names that contain file references (e.g., "gherkin").
    pub file_reference_fields: Vec<String>,
}

pub fn validate(graph: &Graph) -> Vec<Diagnostic> {
    validate_with_config(graph, &ValidatorConfig::default())
}

pub fn validate_with_config(graph: &Graph, config: &ValidatorConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    orphan::detect_orphan_structural_nodes(graph, &mut diagnostics);
    file_ref::validate_file_references(graph, config, &mut diagnostics);
    diagnostics
}
