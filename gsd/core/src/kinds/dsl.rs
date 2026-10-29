//! A kind as data. The model (`spec/*.md`) lists each kind's fields with flags; the tables here are those lists, and
//! `declare` turns them into SDK builder calls, so a field costs one line and the notation of `spec/README.md` stays
//! readable in the code: `str("name").r()` is `name | str | R`.

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
    /// A list of values of a vocabulary, each once (ADR 0285).
    Enums(&'static [&'static str]),
    Strs,
    /// Paths of project files, from the project root: the host's `E016` checks that each exists.
    Files,
    Ref(&'static str),
    Refs(&'static str),
    /// A list of references to any of these kinds (ADR 0285 D5).
    RefsAny(&'static [&'static str]),
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
    /// `U`: only a person writes it (ADR 0289).
    pub human: bool,
    /// `+`: the baseline's list is a prefix of the current one.
    pub append: bool,
    /// The inclusive bounds of an `int`.
    pub range: Option<(i64, i64)>,
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
            human: false,
            append: false,
            range: None,
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
    /// `U`
    pub const fn u(mut self) -> Self {
        self.human = true;
        self
    }
    /// `+`
    pub const fn append(mut self) -> Self {
        self.append = true;
        self
    }
    /// `int{lo..hi}`
    pub const fn range(mut self, lo: i64, hi: i64) -> Self {
        self.range = Some((lo, hi));
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
pub const fn ens(name: &'static str, values: &'static [&'static str]) -> Field {
    Field::new(name, Ty::Enums(values))
}
pub const fn strs(name: &'static str) -> Field {
    Field::new(name, Ty::Strs)
}
pub const fn files(name: &'static str) -> Field {
    Field::new(name, Ty::Files)
}
pub const fn rf(name: &'static str, kind: &'static str) -> Field {
    Field::new(name, Ty::Ref(kind))
}
pub const fn rfs(name: &'static str, kind: &'static str) -> Field {
    Field::new(name, Ty::Refs(kind))
}
pub const fn rfs_any(name: &'static str, kinds: &'static [&'static str]) -> Field {
    Field::new(name, Ty::RefsAny(kinds))
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
    /// A log: an entity the baseline holds stays, with every authored value (K4).
    pub append_only: bool,
    /// `U`: only a person writes the kind's entities (ADR 0289).
    pub human: bool,
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
            append_only: false,
            human: false,
            scoped_by: None,
            fields,
        }
    }
    pub const fn singleton(mut self) -> Self {
        self.singleton = true;
        self
    }
    pub const fn append_only(mut self) -> Self {
        self.append_only = true;
        self
    }
    pub const fn human(mut self) -> Self {
        self.human = true;
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
            if let Some((lo, hi)) = field.range {
                f.minimum(lo).maximum(hi);
            }
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
        Ty::Enums(values) => {
            f.enum_list(&Vocabulary::new(values));
        }
        Ty::Strs => {
            f.field_type(FieldType::StringList);
        }
        Ty::Files => {
            f.field_type(FieldType::StringList)
                .file_reference()
                .file_base(FileBase::Project);
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
        Ty::RefsAny(targets) => {
            f.field_type(FieldType::ReferenceList)
                .target_kinds(targets)
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
    if field.human {
        f.human();
    }
    if field.append {
        f.append_only();
    }
    if !field.doc.is_empty() {
        f.description(field.doc);
    }
}

/// Declares the kinds, the edge of each reference field, and `gsd/E107` over each authored `ts` field and each
/// authored `ts` member of an authored block.
pub fn declare(c: &mut ContributionsBuilder, kinds: &[&Kind]) {
    for kind in kinds {
        c.kind(kind.name, |k| {
            k.description(kind.doc)
                .testable(false)
                .singleton(kind.singleton);
            if kind.append_only {
                k.append_only();
            }
            if kind.human {
                k.human();
            }
            if let Some(scope) = kind.scoped_by {
                k.scoped_by(scope);
            }
            for field in kind.fields {
                // A required human field of a kind an agent may write could never be created (W021).
                assert!(
                    !(field.human && field.required && !kind.human),
                    "{}.{}: a required human field needs a human kind",
                    kind.name,
                    field.name
                );
                k.field(field.name, |f| apply(kind.name, f, field));
            }
        });
        for field in kind.fields {
            declare_edges(c, kind.name, kind.name, field);
            if field.writer == Writer::Author {
                for path in authored_timestamps(field) {
                    c.rule("E107", |r| {
                        r.check(CheckKind::FieldValueConstraint)
                            .target_kind(kind.name)
                            .field(&path)
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
}

/// The edge of a reference field (`owner` is its kind), and of each reference member of a block: `apply` labels a member's edge by its
/// path (`<Kind><Field><Member>`), and the host would otherwise find the label undeclared.
fn declare_edges(c: &mut ContributionsBuilder, kind: &str, owner: &str, field: &Field) {
    match field.ty {
        Ty::Ref(target) | Ty::Refs(target) => {
            c.edge(&edge_label(owner, field.name), |e| {
                e.source_kind(kind).target_kind(target);
            });
        }
        // One edge for the union: an edge has one target kind, so it names none.
        Ty::RefsAny(_) => {
            c.edge(&edge_label(owner, field.name), |e| {
                e.source_kind(kind);
            });
        }
        Ty::Block(members) | Ty::Blocks(members) => {
            let owner = format!("{owner}_{}", field.name);
            for member in members {
                declare_edges(c, kind, &owner, member);
            }
        }
        _ => {}
    }
}

/// The written paths of the authored `ts` values under `field`: the field itself, or `block.member` of a block's
/// members. A member of a computed or harness field is never reached (the caller checks the field's writer).
fn authored_timestamps(field: &Field) -> Vec<String> {
    match field.ty {
        Ty::Ts => vec![field.name.to_string()],
        Ty::Block(members) | Ty::Blocks(members) => members
            .iter()
            .filter(|m| matches!(m.ty, Ty::Ts) && m.writer == Writer::Author)
            .map(|m| format!("{}.{}", field.name, m.name))
            .collect(),
        _ => Vec::new(),
    }
}

/// `YYYY-MM-DDTHH:MM:SS[.fraction]Z`, ASCII digits only, with RFC 3339's ranges (month 01-12, day 01-31, hour 00-23,
/// minute 00-59, second 00-60). The day is not checked against the month (no 30 February test in a regex).
pub const RFC3339_UTC: &str = r"^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)([.][0-9]+)?Z$";

/// The `kind.field` names of every computed top-level field of `kinds`, and the `kind.field.member` names of every
/// computed member of an authored block (one level: the model nests no deeper): what the derive pass lists in `computes`.
pub fn computed(kinds: &[&Kind]) -> Vec<String> {
    let mut found = Vec::new();
    for kind in kinds {
        for field in kind.fields {
            if field.writer == Writer::Computed {
                found.push(format!("{}.{}", kind.name, field.name));
            } else if let Ty::Block(members) | Ty::Blocks(members) = field.ty {
                found.extend(
                    members
                        .iter()
                        .filter(|m| m.writer == Writer::Computed)
                        .map(|m| format!("{}.{}.{}", kind.name, field.name, m.name)),
                );
            }
        }
    }
    found
}

/// The `kind.field` names of every top-level field of `kinds` whose type the kernel hands a pass as structured
/// `values` today (lists and blocks): the only fields the context digest reads from `values`.
pub fn structured(kinds: &[&Kind]) -> Vec<String> {
    kinds
        .iter()
        .flat_map(|kind| {
            kind.fields
                .iter()
                .filter(|f| {
                    matches!(
                        f.ty,
                        Ty::Block(_)
                            | Ty::Blocks(_)
                            | Ty::Strs
                            | Ty::Files
                            | Ty::Refs(_)
                            | Ty::RefsAny(_)
                    )
                })
                .map(move |f| format!("{}.{}", kind.name, f.name))
        })
        .collect()
}

#[cfg(test)]
mod tests;
