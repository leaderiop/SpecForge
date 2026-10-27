//! Derived reference fields (behavior `link_derived_references`).
//!
//! An extension field may declare `derived_from`: the host then gives the
//! field edges from the type names the entity writes in its field types or
//! method signatures, as if the entity had listed them in the field. The
//! core names no kind: which kinds derive which edges comes only from
//! [`DerivedReference`]s, built from the field registry. Each name is read
//! with the position it is written at, so the graph's finder reports it
//! (`written_tokens`, ADR 0176).

use crate::scope::{Found, Scopes};
use crate::{Edge, Graph, Reading};
use specforge_common::{Diagnostic, DiagnosticData, SourceSpan, Sym, codes};
use specforge_parser::lex::{LexemeKind, is_path, lex};
use specforge_parser::{FieldEntry, FieldValue, MethodDecl, Written};

/// The type names that are no entity: what an unknown type name is checked against (E004, ADR 0282 D1), and what the
/// registry sends validators as `context.primitives`. `number`, `integer`, `boolean`, `timestamp` and `date` are the
/// portable primitives docs/entities/type.md documents; `never` marks an impossible error channel
/// (docs/entities/port.md).
pub const PRIMITIVE_TYPES: &[&str] = &[
    "string",
    "void",
    "bool",
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "f32",
    "f64",
    "usize",
    "isize",
    "any",
    "number",
    "integer",
    "boolean",
    "timestamp",
    "date",
    "never",
    "float",
    "object",
    // stdlib containers: their type arguments are names of their own
    "Result",
    "Option",
    "Vec",
    "Box",
    "Arc",
    "Rc",
    "HashMap",
    "HashSet",
    "BTreeMap",
    "BTreeSet",
    "String",
];

/// Where a derived reference field takes its targets from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedFrom {
    /// The type names in the entity's field values written as type syntax:
    /// an identifier, `T[]` or `A | B`, with generics inside them.
    TypeExpressions,
    /// The type names in the entity's method parameter and return types.
    MethodSignatures,
}

impl DerivedFrom {
    /// The `derived_from` value a field descriptor declares.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "type_expressions" => Some(Self::TypeExpressions),
            "method_signatures" => Some(Self::MethodSignatures),
            _ => None,
        }
    }
}

/// One registered field whose edges the host derives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedReference {
    /// The kind that declares the field.
    pub source_kind: String,
    /// The field's name, which labels its edges.
    pub field: String,
    pub from: DerivedFrom,
}

/// One type expression an entity writes, and where it starts: what the derived walk reads names from.
#[derive(Clone, Copy)]
pub(crate) struct TypeExpression<'v> {
    pub text: &'v str,
    /// The field key or method name it is written in.
    pub owner: &'v str,
    /// Whether each name in it must name a type (E004): not a union body's variant nor a list's item, which may be
    /// an enum's words (`type Status = open | closed`, `values [cli, api]`).
    pub typed: bool,
    /// Where `text` starts; `None` for a value built without a position.
    pub at: Option<&'v SourceSpan>,
}

/// The type expressions a field's entry writes, with positions: an identifier's text at the entry's value span, a
/// union's members that were not written quoted at their spans, and a bare item of a list under a member of an
/// open-fields kind at its span. A single reference names an entity, not a type.
pub(crate) fn expressions<'v>(reading: &Reading, entry: &'v FieldEntry) -> Vec<TypeExpression<'v>> {
    if reading.is_single_reference() {
        return Vec::new();
    }
    match &entry.value {
        FieldValue::Identifier(expr) => vec![TypeExpression {
            text: expr,
            owner: entry.key.as_str(),
            typed: true,
            at: entry.value_span.as_ref(),
        }],
        FieldValue::TypeUnion(members) | FieldValue::VariantList(members) => members
            .iter()
            .filter(|m| m.written != Written::Quoted)
            .map(|m| TypeExpression {
                text: &m.text,
                owner: entry.key.as_str(),
                typed: matches!(entry.value, FieldValue::TypeUnion(_)),
                at: Some(&m.span),
            })
            .collect(),
        FieldValue::List(items) if *reading == Reading::Member => items
            .iter()
            .filter(|item| item.written == Written::Bare)
            .map(|item| TypeExpression {
                text: &item.text,
                owner: entry.key.as_str(),
                typed: false,
                at: Some(&item.span),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The type expressions `reference` reads from a block of `kind`: its entries written as types
/// ([`DerivedFrom::TypeExpressions`]), or its methods' parameter and return types
/// ([`DerivedFrom::MethodSignatures`]). The derived walk links the names in them; the finder reports them.
fn type_expressions<'v>(
    reference: &DerivedReference,
    forms: &crate::FieldForms,
    kind: &str,
    entries: &'v [FieldEntry],
    methods: &'v [MethodDecl],
) -> Vec<TypeExpression<'v>> {
    let mut written = Vec::new();
    match reference.from {
        DerivedFrom::TypeExpressions => {
            for entry in entries {
                let reading = forms.reading(kind, entry.key.as_str());
                written.extend(expressions(&reading, entry));
            }
        }
        DerivedFrom::MethodSignatures => {
            for method in methods {
                written.extend(method.params.iter().map(|p| TypeExpression {
                    text: &p.ty,
                    owner: &method.name,
                    typed: true,
                    at: p.ty_span.as_ref(),
                }));
                written.extend(method.returns.as_deref().map(|text| TypeExpression {
                    text,
                    owner: &method.name,
                    typed: true,
                    at: method.returns_span.as_ref(),
                }));
            }
        }
    }
    written
}

/// The names inside a type expression, each with where it is written: the identifiers of the language's lexer
/// (`Result<A, B[]>` -> Result, A, B), at the expression's position plus the identifier's offset.
fn names(expression: TypeExpression<'_>) -> impl Iterator<Item = (&str, Option<SourceSpan>, &str)> {
    // A value or bare item that is one word holding `-` or `.`, or starting with a digit (`Money-v2`, `2-1`, `v1.0`,
    // `007`), or a path of names (`v1.0/03/Money`, ADR 0284), is one name, as the grammar reads it (ADR 0283); a structured type expression keeps the lexer's identifiers.
    let text = expression.text;
    let whole = is_path(text)
        && (text.contains(['-', '.', '/']) || text.starts_with(|c: char| c.is_ascii_digit()));
    let lexemes = if whole {
        vec![(0, text.len())]
    } else {
        lex(text)
            .into_iter()
            .filter(|lexeme| lexeme.kind == LexemeKind::Ident)
            .map(|lexeme| (lexeme.start, lexeme.end))
            .collect()
    };
    lexemes.into_iter().map(move |(start, end)| {
        (
            &text[start..end],
            expression.at.map(|at| within(at, text, start, end)),
            expression.owner,
        )
    })
}

/// One type name that links, as it is written and as it resolves.
pub(crate) struct DerivedName<'v> {
    /// The name or path as written.
    pub written: &'v str,
    /// The entity it names.
    pub path: Sym,
    /// Where it is written (`None`: a value built without a position).
    pub at: Option<SourceSpan>,
}

/// The names `reference` links from one block of its kind, in written order, each with where it is written (`None`: a
/// value built without a position): the type names of the block's type expressions (`entries` as the graph build
/// stores them, and `methods`) that are not the entity `from` itself and name an entity, resolved from `from`'s scope
/// chain (`None`: the project's), that the field's declared target admits. None when the field declares no target.
/// The linker makes each an edge, the finder a token (ADR 0163).
pub(crate) fn derived_names<'v>(
    reference: &DerivedReference,
    forms: &crate::FieldForms,
    entries: &'v [FieldEntry],
    methods: &'v [MethodDecl],
    from: Option<Sym>,
    scopes: &Scopes,
) -> Vec<DerivedName<'v>> {
    let kind = reference.source_kind.as_str();
    let Some(target) = forms.target(kind, &reference.field) else {
        return Vec::new();
    };
    type_expressions(reference, forms, kind, entries, methods)
        .into_iter()
        .flat_map(names)
        .filter_map(|(written, at, _)| {
            let Found::One(path) = scopes.chain(from, written, &|k| target.admits(k)) else {
                return None;
            };
            (Some(path) != from).then_some(DerivedName { written, path, at })
        })
        .collect()
}

/// The names `reference` reads as types from one block of its kind, outside a union body's variants and a list's
/// items (`TypeExpression::typed`), that are not the entity `from` itself, no [`PRIMITIVE_TYPES`] entry,
/// and name no entity (resolved from `from`'s scope chain) the field's target admits: each with where it is written
/// and the field key or method name it is in. The same walk as [`derived_names`], so the linker and the report agree
/// on what a type name is (ADR 0282 D1). None when the field declares no target.
pub(crate) fn unknown_type_names<'v>(
    reference: &DerivedReference,
    forms: &crate::FieldForms,
    entries: &'v [FieldEntry],
    methods: &'v [MethodDecl],
    from: Option<Sym>,
    scopes: &Scopes,
) -> Vec<(&'v str, Option<SourceSpan>, &'v str)> {
    let kind = reference.source_kind.as_str();
    let Some(target) = forms.target(kind, &reference.field) else {
        return Vec::new();
    };
    type_expressions(reference, forms, kind, entries, methods)
        .into_iter()
        .filter(|expression| expression.typed)
        .flat_map(names)
        .filter(|(name, _, _)| {
            // `fn` is a function type's keyword (`fn(A) -> B`), not a name. The entity itself is a known name.
            *name != "fn"
                && !PRIMITIVE_TYPES.contains(name)
                && !matches!(
                    scopes.chain(from, name, &|k| target.admits(k)),
                    Found::One(_)
                )
        })
        .collect()
}

/// The span of bytes `start..end` of `text`, which is written from `at`'s start (lines counted from the newlines
/// before each offset; columns are 1-based bytes).
fn within(at: &SourceSpan, text: &str, start: usize, end: usize) -> SourceSpan {
    let position = |offset: usize| {
        let before = &text.as_bytes()[..offset];
        match before.iter().rposition(|&b| b == b'\n') {
            None => (at.start_line, at.start_col + offset),
            Some(newline) => (
                at.start_line + before.iter().filter(|&&b| b == b'\n').count(),
                offset - newline,
            ),
        }
    };
    let (start_line, start_col) = position(start);
    let (end_line, end_col) = position(end);
    SourceSpan {
        file: at.file,
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

impl Graph {
    /// Add the edges of every derived reference field, and report each name read as a type that names no type
    /// (E004, ADR 0282 D1). Idempotent; run after references resolve, since resolution clears the edges.
    pub(crate) fn link_derived_references(
        &mut self,
        derived: &[DerivedReference],
        forms: &crate::FieldForms,
    ) -> Vec<Diagnostic> {
        let mut edges: Vec<Edge> = Vec::new();
        let mut diagnostics = Vec::new();
        for reference in derived {
            for node in self.nodes_by_kind(&reference.source_kind) {
                let names = derived_names(
                    reference,
                    forms,
                    node.fields.entries(),
                    &node.methods,
                    Some(node.id.raw),
                    self.scopes(),
                );
                edges.extend(names.into_iter().map(|name| Edge {
                    source: node.id.raw,
                    target: name.path,
                    label: Sym::new(&reference.field),
                }));
                let unknown = unknown_type_names(
                    reference,
                    forms,
                    node.fields.entries(),
                    &node.methods,
                    Some(node.id.raw),
                    self.scopes(),
                );
                for (name, at, owner) in unknown {
                    let place = match reference.from {
                        DerivedFrom::TypeExpressions => "field",
                        DerivedFrom::MethodSignatures => "method",
                    };
                    let mut diagnostic = Diagnostic::new(
                        codes::E004,
                        format!(
                            "{} '{}' {place} '{owner}' references unknown type '{name}'",
                            node.kind.raw, node.id.raw
                        ),
                    )
                    .with_data(DiagnosticData::UnresolvedReference {
                        target: name.to_string(),
                        entity: node.id.raw.to_string(),
                        field: owner.to_string(),
                        did_you_mean: None,
                        candidates: Vec::new(),
                    })
                    .with_suggestion(format!(
                        "declare a type '{name}', or name a declared type or a primitive"
                    ));
                    if let Some(at) = at {
                        diagnostic = diagnostic.with_span(at);
                    }
                    diagnostics.push(diagnostic);
                }
            }
        }
        for edge in edges {
            self.add_edge(edge);
        }
        diagnostics
    }
}
