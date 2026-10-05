mod resolve;

pub use resolve::{
    PathAlias, Resolution, ResolveConfig, resolve_import, resolve_parsed, resolve_project,
    resolve_project_with_config,
};
pub use specforge_common::{Diagnostic, Severity, SourceSpan};
pub use specforge_parser::{
    Entity, FieldValue, ImportBinding, ImportDeclaration, ImportKind, SpecFile,
};

use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct ResolvedProject {
    pub files: Vec<ResolvedFile>,
    pub diagnostics: Vec<Diagnostic>,
    pub file_scopes: HashMap<String, FileScope>,
}

impl ResolvedProject {
    /// Each file's text, by its path relative to the spec root: exactly
    /// what was parsed, for quoting in rendered diagnostics without
    /// reading the disk again.
    pub fn source_texts(&self) -> HashMap<String, String> {
        self.files
            .iter()
            .map(|file| (file.path.clone(), file.source.clone()))
            .collect()
    }
}

#[derive(Debug)]
pub struct ResolvedFile {
    pub path: String,
    /// The text this file was parsed from.
    pub source: String,
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
