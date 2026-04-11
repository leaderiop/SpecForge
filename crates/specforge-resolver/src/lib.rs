pub mod linker;
mod resolve;

pub use resolve::{resolve_project, resolve_project_with_config, PathAlias, ResolveConfig};
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{Entity, FieldValue, ImportBinding, ImportDeclaration, ImportKind, SpecFile};

use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct ResolvedProject {
    pub files: Vec<ResolvedFile>,
    pub diagnostics: Vec<Diagnostic>,
    pub file_scopes: HashMap<String, FileScope>,
}

#[derive(Debug)]
pub struct ResolvedFile {
    pub path: String,
    pub spec_file: SpecFile,
    pub import_targets: Vec<String>,
    pub reexports: Vec<ReexportDeclaration>,
}

#[derive(Debug, Clone)]
pub struct ReexportDeclaration {
    pub target_path: String,
    pub bindings: Option<Vec<ImportBinding>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default)]
pub struct FileScope {
    pub declared: HashSet<String>,
    pub exported: HashSet<String>,
}
