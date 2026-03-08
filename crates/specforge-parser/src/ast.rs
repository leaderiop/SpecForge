use specforge_common::SourceSpan;

#[derive(Debug, Clone)]
pub struct SpecFile {
    pub path: String,
    pub imports: Vec<ImportDeclaration>,
    pub entities: Vec<Entity>,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone)]
pub struct ImportDeclaration {
    pub path: String,
    pub selected_ids: Option<Vec<String>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: EntityKind,
    pub id: EntityId,
    pub title: Option<String>,
    pub fields: FieldMap,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityKind {
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityId {
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct FieldMap {
    entries: Vec<FieldEntry>,
}

impl FieldMap {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn push(&mut self, key: String, value: FieldValue) {
        self.entries.push(FieldEntry { key, value });
    }

    pub fn get(&self, key: &str) -> Option<&FieldValue> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    pub fn entries(&self) -> &[FieldEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone)]
pub struct FieldEntry {
    pub key: String,
    pub value: FieldValue,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    ReferenceList(Vec<String>),
    StringList(Vec<String>),
    Block(FieldMap),
    VerifyList(Vec<VerifyStatement>),
    Integer(i64),
    Boolean(bool),
    Date(String),
    Identifier(String),
}

#[derive(Debug, Clone)]
pub struct VerifyStatement {
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: SourceSpan,
    pub expected: Option<String>,
    pub found: Option<String>,
}
