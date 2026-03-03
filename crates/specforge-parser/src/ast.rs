use specforge_common::{CustomEntityDef, EntityId, EntityKind, FieldMap, SourceSpan};

/// A parsed `.spec` file.
#[derive(Debug, Clone)]
pub struct SpecFile {
    pub path: String,
    pub imports: Vec<UseImport>,
    pub entities: Vec<AstEntity>,
    pub custom_defs: Vec<CustomEntityDef>,
    pub errors: Vec<ParseError>,
}

/// A `use` import declaration.
#[derive(Debug, Clone)]
pub struct UseImport {
    pub path: String,
    pub selective: Option<Vec<String>>,
    pub span: SourceSpan,
}

/// A parsed entity block in the AST.
#[derive(Debug, Clone)]
pub struct AstEntity {
    pub kind: EntityKind,
    pub id: EntityId,
    pub title: Option<String>,
    pub fields: FieldMap,
    pub span: SourceSpan,
}

/// A parse error with source location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: SourceSpan,
}
