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
        self.entries.push(FieldEntry { key, value });
    }

    pub fn get(&self, key: &str) -> Option<&FieldValue> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    pub fn entries(&self) -> &[FieldEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldEntry {
    pub key: Sym,
    pub value: FieldValue,
}

#[derive(Debug, Clone, Serialize)]
pub enum FieldValue {
    String(String),
    ReferenceList(Vec<String>),
    VariantList(Vec<String>),
    StringList(Vec<String>),
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
