mod linker;
mod resolve;

pub use linker::{link_references, PendingEdge};
pub use resolve::resolve_project;
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{Entity, FieldValue, ImportDeclaration, SpecFile};

#[derive(Debug)]
pub struct ResolvedProject {
    pub files: Vec<ResolvedFile>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct ResolvedFile {
    pub path: String,
    pub spec_file: SpecFile,
    pub import_targets: Vec<String>,
}
