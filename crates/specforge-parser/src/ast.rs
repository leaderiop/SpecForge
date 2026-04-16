use serde::Serialize;
use specforge_common::{SourceSpan, Sym};

#[derive(Debug, Clone, Serialize)]
pub struct SpecFile {
    pub path: Sym,
    pub imports: Vec<ImportDeclaration>,
    pub entities: Vec<Entity>,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ImportKind {
    Full,
    Selective,
    Namespace,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportBinding {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDeclaration {
    pub path: Sym,
    pub kind: ImportKind,
    pub bindings: Option<Vec<ImportBinding>>,
    pub namespace: Option<String>,
    pub is_pub: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    pub kind: EntityKind,
    pub id: EntityId,
    pub title: Option<String>,
    pub fields: FieldMap,
    pub raw_body: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct EntityKind {
    pub raw: Sym,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct EntityId {
    pub raw: Sym,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldMap {
    entries: Vec<FieldEntry>,
}

impl Default for FieldMap {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldMap {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn push(&mut self, key: Sym, value: FieldValue) {
        self.entries.push(FieldEntry { key, value, annotations: Vec::new() });
    }

    pub fn push_annotated(&mut self, key: Sym, value: FieldValue, annotations: Vec<Annotation>) {
        self.entries.push(FieldEntry { key, value, annotations });
    }

    pub fn get(&self, key: &str) -> Option<&FieldValue> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    pub fn entries(&self) -> &[FieldEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Annotation {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldEntry {
    pub key: Sym,
    pub value: FieldValue,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Clone, Serialize)]
pub enum FieldValue {
    String(String),
    ReferenceList(Vec<String>),
    VariantList(Vec<String>),
    StringList(Vec<String>),
    /// A list containing items of mixed types (e.g., strings, integers, booleans).
    /// Preserves per-item type information instead of flattening to StringList.
    MixedList(Vec<FieldValue>),
    Block(FieldMap),
    VerifyList(Vec<VerifyStatement>),
    Integer(i64),
    Boolean(bool),
    Date(String),
    Identifier(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyStatement {
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseError {
    pub message: String,
    pub span: SourceSpan,
    pub expected: Option<String>,
    pub found: Option<String>,
}

impl From<&ParseError> for specforge_common::Diagnostic {
    fn from(err: &ParseError) -> Self {
        // Parse errors are warnings, not errors: tree-sitter recovers partial ASTs,
        // so entities from files with syntax issues are still usable. The grammar
        // is intentionally permissive and doesn't cover all valid spec syntax.
        specforge_common::Diagnostic::warning("W062", &err.message)
            .with_span(err.span.clone())
    }
}
