//! The `.spec` language read from text: the one lexer (`lex`), the recovered syntax (`cst`), the AST the parser
//! walks it into (`ast`, `parse`), the expression grammar (`expr`) and the format version header
//! (`format_version`) and what a line break is (`line_break`). Every reader of `.spec` text goes through it (ADR 0038, ADR 0063).

pub mod ast;
pub mod cst;
pub mod expr;
mod format_version;
pub mod lex;
pub mod line_break;
mod parse;
#[cfg(feature = "testing")]
pub mod testing;

pub use ast::{
    Annotation, Entity, EntityId, EntityKind, FieldEntry, FieldMap, FieldValue, ImportBinding,
    ImportDeclaration, ImportKind, ListItem, MethodDecl, Parameter, ParseError, REF_SCHEME_FIELD,
    Repeat, SpecFile, TypeMember, UNION_VARIANTS_FIELD, VerifyStatement, Written,
};
pub use expr::{CmpOp, Expr, ExprError, ExprSpan, SpannedExpr, parse_expression};
pub use format_version::{
    CURRENT_FORMAT_VERSION, FormatVersion, MAX_SUPPORTED_VERSION, MIN_SUPPORTED_VERSION,
    detect_format_version, with_format_version,
};
pub use line_break::LineBreak;
pub use parse::{Previous, parse, parse_incremental, value_token};
