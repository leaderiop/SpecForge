//! A kind as data. The model (`spec/*.md`) lists each kind's fields with flags; the tables here are those lists, and
//! `declare` turns them into SDK builder calls, so a field costs one line and the notation of `spec/README.md` stays
//! readable in the code: `str("name").r()` is `name | str | R`.

#![expect(
    dead_code,
    reason = "the tables of this round's later tickets use the rest of the notation; the last of them removes this"
)]

use specforge_extension_sdk::prelude::*;

/// A field's type, in the notation of `spec/README.md`.
#[derive(Clone, Copy)]
pub enum Ty {
    Str,
    Int,
    Bool,
    Date,
    /// An RFC 3339 UTC time: a string that `gsd/E107` checks when it is authored.
    Ts,
    Enum(&'static [&'static str]),
    Strs,
    Ref(&'static str),
    Refs(&'static str),
    Block(&'static [Field]),
    Blocks(&'static [Field]),
}

/// Who writes a field.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Writer {
    /// A human or an agent, through an edit plan (no flag).
    Author,
    /// `C`: a compute pass.
    Computed,
    /// `H`: a command's or collector's plan.
    Harness,
}

#[derive(Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub ty: Ty,
    pub required: bool,
    pub writer: Writer,
    pub immutable: bool,
    /// The name the target kind reads this reference by (`lessons` for `lesson.retrospective`).
    pub inverse: &'static str,
    pub doc: &'static str,
}

impl Field {
    const fn new(name: &'static str, ty: Ty) -> Self {
        Field {
            name,
            ty,
            required: false,
            writer: Writer::Author,
            immutable: false,
            inverse: "",
            doc: "",
        }
    }
    /// `R`
    pub const fn r(mut self) -> Self {
        self.required = true;
        self
    }
    /// `I`
    pub const fn i(mut self) -> Self {
        self.immutable = true;
        self
    }
    /// `C`
    pub const fn c(mut self) -> Self {
        self.writer = Writer::Computed;
        self
    }
    /// `H`
    pub const fn h(mut self) -> Self {
        self.writer = Writer::Harness;
        self
    }
    /// The name the target kind reads this relation by.
    pub const fn inverse(mut self, name: &'static str) -> Self {
        self.inverse = name;
        self
    }
    /// What the field is, in a few words (the GSD notation it replaces).
    pub const fn doc(mut self, doc: &'static str) -> Self {
        self.doc = doc;
        self
    }
}

pub const fn str(name: &'static str) -> Field {
    Field::new(name, Ty::Str)
}
pub const fn int(name: &'static str) -> Field {
    Field::new(name, Ty::Int)
}
pub const fn bool(name: &'static str) -> Field {
    Field::new(name, Ty::Bool)
}
pub const fn date(name: &'static str) -> Field {
    Field::new(name, Ty::Date)
}
pub const fn ts(name: &'static str) -> Field {
    Field::new(name, Ty::Ts)
}
pub const fn en(name: &'static str, values: &'static [&'static str]) -> Field {
    Field::new(name, Ty::Enum(values))
}
pub const fn strs(name: &'static str) -> Field {
    Field::new(name, Ty::Strs)
}
pub const fn rf(name: &'static str, kind: &'static str) -> Field {
    Field::new(name, Ty::Ref(kind))
}
pub const fn rfs(name: &'static str, kind: &'static str) -> Field {
    Field::new(name, Ty::Refs(kind))
}
pub const fn block(name: &'static str, members: &'static [Field]) -> Field {
    Field::new(name, Ty::Block(members))
}
pub const fn blocks(name: &'static str, members: &'static [Field]) -> Field {
    Field::new(name, Ty::Blocks(members))
}

/// An entity kind: its keyword is its name.
pub struct Kind {
    pub name: &'static str,
    pub doc: &'static str,
    pub singleton: bool,
    /// The reference field that scopes the kind's names (K2).
    pub scoped_by: Option<&'static str>,
    pub fields: &'static [Field],
}

impl Kind {
    pub const fn new(name: &'static str, doc: &'static str, fields: &'static [Field]) -> Self {
        Kind {
            name,
            doc,
            singleton: false,
            scoped_by: None,
            fields,
        }
    }
    pub const fn singleton(mut self) -> Self {
        self.singleton = true;
        self
    }
    pub const fn scoped_by(mut self, field: &'static str) -> Self {
        self.scoped_by = Some(field);
        self
    }
}

fn pascal(word: &str) -> String {
    word.split('_')
        .map(|part| {
            part.chars().next().map_or_else(String::new, |c| {
                c.to_uppercase().chain(part.chars().skip(1)).collect()
            })
        })
        .collect()
}

/// The edge a reference field writes: `<Kind><Field>`, e.g. `PhaseMilestone`.
fn edge_label(kind: &str, field: &str) -> String {
    format!("{}{}", pascal(kind), pascal(field))
}

fn apply(kind: &str, f: &mut FieldBuilder, field: &Field) {
    match field.ty {
        Ty::Str | Ty::Ts => {
            f.field_type(FieldType::String);
        }
        Ty::Int => {
            f.field_type(FieldType::Integer);
        }
        Ty::Bool => {
            f.field_type(FieldType::Bool);
        }
        Ty::Date => {
            f.field_type(FieldType::Date);
        }
        Ty::Enum(values) => {
            f.vocabulary(&Vocabulary::new(values));
        }
        Ty::Strs => {
            f.field_type(FieldType::StringList);
        }
        Ty::Ref(target) => {
            f.field_type(FieldType::Reference)
                .target_kind(target)
                .edge(&edge_label(kind, field.name));
        }
        Ty::Refs(target) => {
            f.field_type(FieldType::ReferenceList)
                .target_kind(target)
                .edge(&edge_label(kind, field.name));
        }
        Ty::Block(members) | Ty::Blocks(members) => {
            let list = matches!(field.ty, Ty::Blocks(_));
            f.field_type(if list {
                FieldType::BlockList
            } else {
                FieldType::Block
            });
            for member in members {
                f.member(member.name, |m| {
                    apply(&format!("{kind}_{}", field.name), m, member)
                });
            }
        }
    }
    if field.required {
        assert!(
            field.writer == Writer::Author,
            "{kind}.{}: a computed or harness field is not required",
            field.name
        );
        f.required();
    }
    match field.writer {
        Writer::Author => {}
        Writer::Computed => {
            f.computed();
        }
        Writer::Harness => {
            f.harness();
        }
    }
    if !field.inverse.is_empty() {
        f.inverse_of(field.inverse);
    }
    if field.immutable {
        f.immutable();
    }
    if !field.doc.is_empty() {
        f.description(field.doc);
    }
}

/// Declares the kinds, the edge of each reference field, and `gsd/E107` over each authored `ts` field.
pub fn declare(c: &mut ContributionsBuilder, kinds: &[&Kind]) {
    for kind in kinds {
        c.kind(kind.name, |k| {
            k.description(kind.doc)
                .testable(false)
                .singleton(kind.singleton);
            if let Some(scope) = kind.scoped_by {
                k.scoped_by(scope);
            }
            for field in kind.fields {
                k.field(field.name, |f| apply(kind.name, f, field));
            }
        });
        for field in kind.fields {
            if let Ty::Ref(target) | Ty::Refs(target) = field.ty {
                c.edge(&edge_label(kind.name, field.name), |e| {
                    e.source_kind(kind.name).target_kind(target);
                });
            }
            if matches!(field.ty, Ty::Ts) && field.writer == Writer::Author {
                c.rule("E107", |r| {
                    r.check(CheckKind::FieldValueConstraint)
                        .target_kind(kind.name)
                        .field(field.name)
                        .constraint(|fc| {
                            fc.kind(ConstraintKind::Matches).pattern(RFC3339_UTC);
                        })
                        .severity(ValidationSeverity::Error)
                        .message_template("{kind} '{id}' has '{value}' where a time like 2026-10-28T09:30:00Z belongs");
                });
            }
        }
    }
}

/// `YYYY-MM-DDTHH:MM:SS[.fraction]Z`.
pub const RFC3339_UTC: &str = r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$";

/// The `kind.field` names of every computed top-level field of `kinds`: what the derive pass lists in `computes`.
pub fn computed(kinds: &[&Kind]) -> Vec<String> {
    kinds
        .iter()
        .flat_map(|kind| {
            kind.fields
                .iter()
                .filter(|f| f.writer == Writer::Computed)
                .map(move |f| format!("{}.{}", kind.name, f.name))
        })
        .collect()
}
