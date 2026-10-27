//! A declared field type's one reading (ADR 0064, ADR 0093): which written values a
//! field takes, how the graph build stores a written value, the JSON every
//! output writes for a field value, and the JSON Schema of that JSON. The
//! graph build's coercion, E061, the export writer, the graph delta and the
//! published schema read it; nothing else decides a field value's form.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_common::find_close_match;
use specforge_common::shape::{Definitions, Shape};
use specforge_parser::{FieldEntry, FieldValue, ListItem, Written};
use specforge_protocol_types::{FieldType, FileBase};

use crate::Graph;

pub mod json;
pub mod members;
pub mod targets;
pub mod word;
pub(crate) use json::write_fields;
pub use json::{FieldJson, write};
pub use targets::Targets;
use word::dashed_name;
pub use word::{integer_difference, is_word};
pub mod spell;
pub use members::{BlockMembers, Member, MemberPath, declared_path};
pub use spell::{Place, Respell, Rewrite};

/// What a field's value was written as, after the graph build coerced it:
/// one variant per parsed value form, so a check tells `"1"` from `1`, a
/// one-item list from a scalar and a quoted string from a bare word
/// (ADR 0019, ADR 0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    /// `"text"`.
    String,
    /// A bare word: an identifier, or a single reference.
    Identifier,
    Date,
    Integer,
    Boolean,
    /// `[a, "b", 1]`: a list, however its items were written.
    List,
    /// A variant list (`a | b` in a body).
    Variants,
    TypeUnion,
    Block,
    /// `[ { … } { … } ]`: a list of blocks (ADR 0285).
    Blocks,
    /// An `expr { … }` group.
    Expression,
}

impl ValueShape {
    /// The shape of `value`: one per `FieldValue` variant, no `_` arm, so a
    /// new variant does not compile until it has one.
    pub fn of(value: &FieldValue) -> Self {
        match value {
            FieldValue::String(_) => Self::String,
            FieldValue::Identifier(_) => Self::Identifier,
            FieldValue::Date(_) => Self::Date,
            FieldValue::Integer(_) => Self::Integer,
            FieldValue::Boolean(_) => Self::Boolean,
            FieldValue::List(_) => Self::List,
            FieldValue::VariantList(_) => Self::Variants,
            FieldValue::TypeUnion(_) => Self::TypeUnion,
            FieldValue::Block(_) => Self::Block,
            FieldValue::Blocks(_) => Self::Blocks,
            FieldValue::Expression(_) => Self::Expression,
        }
    }

    /// A list, a variant list or a list of blocks.
    pub fn is_list(self) -> bool {
        matches!(self, Self::List | Self::Variants | Self::Blocks)
    }
}

/// A field's declared type and, for an enum, its declared values: which
/// stored values the field takes ([`Self::admit`]), how the graph build
/// stores a written value ([`Self::coerce`]), and the JSON Schema of what
/// [`write`] writes for every value it takes ([`Self::json_schema`]).
///
/// Invariant (the module's property test): for every written value `v`,
/// with `stored = coerce(v).unwrap_or(v)`, `admit(ValueShape::of(&stored),
/// text)` is `Ok` only if `write(&stored)` conforms to `json_schema()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldForm {
    field_type: FieldType,
    /// Empty unless `field_type` is `Enum` or `EnumList`.
    enum_values: Vec<String>,
    /// The kinds its references must name (the descriptor's `target_kind` or `target_kinds`); empty when it
    /// declares none.
    targets: Vec<String>,
    /// A typed `block` or `block_list`'s members; `None` for an open block and every other type.
    members: Option<Arc<BlockMembers>>,
    /// An integer's least and greatest value, inclusive.
    bounds: (Option<i64>, Option<i64>),
    /// Where a file field's relative paths start.
    base: FileBase,
    /// A member's `required`: present in each written item. The top level keeps E006's host rule.
    required: bool,
}

/// Why a field does not take a value: the remedy E061 suggests, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub suggestion: Option<String>,
}

impl Refusal {
    fn with(suggestion: &str) -> Self {
        Self {
            suggestion: Some(suggestion.to_string()),
        }
    }

    fn none() -> Self {
        Self { suggestion: None }
    }
}

impl FieldForm {
    /// `enum_values` are an enum's or enum list's declared values; any other type drops them.
    pub fn new(field_type: FieldType, enum_values: Vec<String>) -> Self {
        let enum_values = match field_type {
            FieldType::Enum | FieldType::EnumList => enum_values,
            _ => Vec::new(),
        };
        Self {
            field_type,
            enum_values,
            targets: Vec::new(),
            members: None,
            bounds: (None, None),
            base: FileBase::Spec,
            required: false,
        }
    }

    /// The same form, whose references must name `kind` entities (ADR 0117).
    pub fn targeting(self, kind: &str) -> Self {
        self.targeting_any(&[kind.to_string()])
    }

    /// The same form, whose references may name an entity of any of `kinds` (ADR 0285).
    pub fn targeting_any(mut self, kinds: &[String]) -> Self {
        self.targets = kinds.to_vec();
        self
    }

    /// The same form, typed by `members` (a `block` or `block_list`).
    pub fn with_members(mut self, members: BlockMembers) -> Self {
        self.members = Some(Arc::new(members));
        self
    }

    /// The same form, whose integers lie within `minimum` and `maximum`, inclusive.
    pub fn bounded(mut self, minimum: Option<i64>, maximum: Option<i64>) -> Self {
        self.bounds = (minimum, maximum);
        self
    }

    /// The same form, whose file paths start at `base`.
    pub fn based(mut self, base: FileBase) -> Self {
        self.base = base;
        self
    }

    /// The same form, required in each written item of the block it is a member of.
    pub fn requiring(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// The kind its references must name, if its descriptor declares exactly one. Whether a loaded
    /// extension declares that kind is [`FieldForms::target`]'s answer. (A union is read from T6 on.)
    pub fn target_kind(&self) -> Option<&str> {
        match self.targets.as_slice() {
            [kind] => Some(kind),
            _ => None,
        }
    }

    /// The kinds its references may name: one, or the union; empty when it declares none.
    pub fn target_kinds(&self) -> &[String] {
        &self.targets
    }

    /// A typed block's or block list's members; `None` for an open block and every other type.
    pub fn members(&self) -> Option<&BlockMembers> {
        self.members.as_deref()
    }

    /// An integer's least and greatest value, inclusive.
    pub fn bounds(&self) -> (Option<i64>, Option<i64>) {
        self.bounds
    }

    /// Where a file field's relative paths start.
    pub fn base(&self) -> FileBase {
        self.base
    }

    /// Whether a member is required in each written item.
    pub fn required(&self) -> bool {
        self.required
    }

    pub fn field_type(&self) -> FieldType {
        self.field_type
    }

    /// An enum's declared values; empty for every other type.
    pub fn enum_values(&self) -> &[String] {
        &self.enum_values
    }

    /// What the graph build stores for `value` when it is not stored as
    /// written; `None` keeps the parsed value. Never diagnoses (what still
    /// cannot be the type is E061's); idempotent. `span` is the value's own,
    /// which a single reference keeps when it becomes a one-item list.
    ///
    /// - `string_list`, `reference_list`: a list is stored as written; a
    ///   bare word, string, integer, boolean or date becomes the one-item
    ///   list `field [value]` parses to.
    /// - `integer`: a quoted integer, a bare word that is an integer difference (`2-1`, `007`) or a date
    ///   (`2026-10-26`, the difference `1990`) becomes the integer ([`integer_difference`]).
    /// - `bool`: a quoted or bare `true` / `false` becomes the boolean.
    /// - `date`: a quoted calendar day becomes the date.
    /// - `string`, `enum`: an integer or boolean becomes its text.
    /// - `reference`: a number, date or boolean written bare becomes the name it is written as (`10`, `2026-10-26`);
    ///   a negative number stays, for E061.
    /// - `enum_list`: as the other lists.
    /// - `block`, `block_list`: each member of a typed block is stored as its own form coerces it, a single block
    ///   on a `block_list` is its one item and `[]` is no item ([`members::coerce_members`]); an open block is
    ///   stored as written.
    pub fn coerce(&self, value: &FieldValue, span: Option<&SourceSpan>) -> Option<FieldValue> {
        let own = self.coerce_own(value, span);
        match members::coerce_members(self, own.as_ref().unwrap_or(value), span) {
            Some(inner) => Some(inner),
            None => own,
        }
    }

    fn coerce_own(&self, value: &FieldValue, span: Option<&SourceSpan>) -> Option<FieldValue> {
        match (self.field_type, value) {
            (FieldType::StringList | FieldType::ReferenceList | FieldType::EnumList, value) => {
                coerce_list(value, span)
            }
            (FieldType::Integer, FieldValue::String(s)) => s
                .trim()
                .parse::<i64>()
                .ok()
                .or_else(|| integer_difference(s).ok())
                .map(FieldValue::Integer),
            (FieldType::Integer, FieldValue::Identifier(s) | FieldValue::Date(s)) => {
                integer_difference(s).ok().map(FieldValue::Integer)
            }
            (FieldType::Reference, FieldValue::Integer(n)) if *n >= 0 => {
                Some(FieldValue::Identifier(n.to_string()))
            }
            (FieldType::Reference, FieldValue::Date(s)) => Some(FieldValue::Identifier(s.clone())),
            (FieldType::Reference, FieldValue::Boolean(b)) => {
                Some(FieldValue::Identifier(b.to_string()))
            }
            (FieldType::Bool, FieldValue::String(s) | FieldValue::Identifier(s)) => {
                match s.as_str() {
                    "true" => Some(FieldValue::Boolean(true)),
                    "false" => Some(FieldValue::Boolean(false)),
                    _ => None,
                }
            }
            (FieldType::Date, FieldValue::String(s)) if spell::is_date(s) => {
                Some(FieldValue::Date(s.clone()))
            }
            (
                FieldType::String | FieldType::Enum | FieldType::Identifier,
                FieldValue::Integer(n),
            ) => Some(FieldValue::String(n.to_string())),
            (
                FieldType::String | FieldType::Enum | FieldType::Identifier,
                FieldValue::Boolean(b),
            ) => Some(FieldValue::String(b.to_string())),
            _ => None,
        }
    }

    /// Whether a stored value of `shape`, whose field text is `text`, is one
    /// this field takes. `Err` carries E061's remedy.
    ///
    /// | declared | takes |
    /// |---|---|
    /// | `string`, `enum` without values | text, a bare word, a date, a type union, an expression group |
    /// | `enum` | a string or bare word that is a declared value |
    /// | `identifier` | one word (letters, digits and `_`, starting with a letter or `_`), bare or quoted |
    /// | `integer` | an integer, a bare integer difference (`2-1`) or a date (a quoted one is unquoted) |
    /// | `bool` | `true` or `false` |
    /// | `date` | one calendar day, `YYYY-MM-DD`, that exists |
    /// | `reference` | a bare word, a number or a date written as a name, or a string; never one starting with `-` |
    /// | `string_list`, `reference_list` | a list, however its items were written |
    /// | `block` | a block |
    pub fn admit(&self, shape: ValueShape, text: &str) -> Result<(), Refusal> {
        let single_value = || Refusal::with("give a single value, not a list");
        match self.field_type {
            FieldType::Integer => match shape {
                ValueShape::Integer => self.in_bounds(text),
                ValueShape::Identifier => {
                    Err(integer_difference(text).err().unwrap_or_else(Refusal::none))
                }
                _ => Err(Refusal::none()),
            },
            FieldType::Bool => match shape {
                ValueShape::Boolean => Ok(()),
                _ => Err(Refusal::with("use true or false")),
            },
            FieldType::Date => match shape {
                ValueShape::Date if spell::is_date(text) => Ok(()),
                ValueShape::Date => Err(Refusal::with(
                    "there is no such day: write a calendar day, YYYY-MM-DD",
                )),
                s if s.is_list() => Err(single_value()),
                _ => Err(Refusal::with("write a calendar day, YYYY-MM-DD")),
            },
            FieldType::Enum if !self.enum_values.is_empty() => {
                let values = &self.enum_values;
                let one_of = || Refusal::with(&format!("use one of: {}", values.join(", ")));
                match shape {
                    ValueShape::String | ValueShape::Identifier
                        if values.iter().any(|v| v == text) =>
                    {
                        Ok(())
                    }
                    ValueShape::String | ValueShape::Identifier => Err(Refusal::with(
                        &match find_close_match(text, values.iter().map(String::as_str)) {
                            Some(close) => format!("did you mean '{close}'?"),
                            None => format!("use one of: {}", values.join(", ")),
                        },
                    )),
                    _ => Err(one_of()),
                }
            }
            FieldType::String | FieldType::Enum => match shape {
                ValueShape::String
                | ValueShape::Identifier
                | ValueShape::Date
                | ValueShape::TypeUnion
                | ValueShape::Expression => Ok(()),
                s if s.is_list() => Err(single_value()),
                _ => Err(Refusal::with("write the value as text")),
            },
            FieldType::Identifier => match shape {
                ValueShape::String | ValueShape::Identifier if is_word(text) => Ok(()),
                s if s.is_list() => Err(single_value()),
                _ => Err(Refusal::with(
                    "write one word: letters, digits and _, starting with a letter or _",
                )),
            },
            FieldType::Reference => match shape {
                s if s.is_list() => Err(single_value()),
                _ if text.starts_with('-') => Err(dashed_name()),
                ValueShape::Identifier | ValueShape::String => Ok(()),
                _ => Err(Refusal::with("name one entity by its id")),
            },
            FieldType::StringList | FieldType::ReferenceList | FieldType::EnumList => match shape {
                ValueShape::List | ValueShape::Variants => Ok(()),
                _ => Err(Refusal::with("write a list: [a, b]")),
            },
            FieldType::BlockList => match shape {
                ValueShape::Blocks => Ok(()),
                ValueShape::List | ValueShape::Variants => {
                    Err(Refusal::with("write each item as a block: { name value }"))
                }
                _ => Err(Refusal::with("write a list of blocks: [ { name value } ]")),
            },
            FieldType::Block => match shape {
                ValueShape::Block => Ok(()),
                _ => Err(Refusal::with("write a block: { name value }")),
            },
        }
    }

    /// The first item of a list field that is none of its declared type: on a `reference_list`, one starting with
    /// `-`; on an `enum_list`, one that is no declared value or repeats an earlier one. `Err` is that item's text and
    /// the refusal.
    pub fn refuse_item<'a>(&self, items: &'a [String]) -> Option<(&'a str, Refusal)> {
        match self.field_type {
            FieldType::ReferenceList => items
                .iter()
                .find(|item| item.starts_with('-'))
                .map(|item| (item.as_str(), dashed_name())),
            FieldType::EnumList if !self.enum_values.is_empty() => {
                items.iter().enumerate().find_map(|(i, item)| {
                    if !self.enum_values.contains(item) {
                        let values = self.enum_values.iter().map(String::as_str);
                        let advice = match find_close_match(item, values) {
                            Some(close) => format!("did you mean '{close}'?"),
                            None => format!("use one of: {}", self.enum_values.join(", ")),
                        };
                        Some((item.as_str(), Refusal::with(&advice)))
                    } else if items[..i].contains(item) {
                        Some((
                            item.as_str(),
                            Refusal::with(&format!("'{item}' is written twice: each value once")),
                        ))
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }

    /// `Ok` when the integer `text` lies within the declared bounds; E061's remedy names them.
    fn in_bounds(&self, text: &str) -> Result<(), Refusal> {
        let (minimum, maximum) = self.bounds;
        let Ok(n) = text.trim().parse::<i64>() else {
            return Ok(());
        };
        if minimum.is_some_and(|m| n < m) || maximum.is_some_and(|m| n > m) {
            return Err(Refusal::with(&match (minimum, maximum) {
                (Some(low), Some(high)) => format!("write a whole number from {low} to {high}"),
                (Some(low), None) => format!("write a whole number of at least {low}"),
                (None, Some(high)) => format!("write a whole number of at most {high}"),
                (None, None) => unreachable!("an unbounded integer is in bounds"),
            }));
        }
        Ok(())
    }

    /// The JSON Schema of what [`write`] writes for every value
    /// [`Self::admit`] takes: `{"type":"string"}` for `string`, `reference`
    /// and a valueless `enum`; `{"type":"string","enum":[…]}` for an `enum`;
    /// `{"type":"integer"}`; `{"type":"boolean"}`; `{"type":"string","format":"date"}` for a `date`;
    /// `{"type":"array","items":{"type":"string"}}` for both lists; an `enum_list` adds its `enum` and
    /// `uniqueItems`; an `integer` its `minimum` and `maximum`;
    /// `{"type":"object","additionalProperties":<FieldJson>}` for an open `block`
    /// (its entries are undeclared values), `FieldJson` defined in `defs`; a typed `block` an object closed to its
    /// members; a `block_list` an array of those (ADR 0285).
    pub fn json_schema(&self, defs: &mut Definitions) -> Value {
        match self.field_type {
            // An identifier's one-word rule is `admit`'s: the in-repo schema checker reads no `pattern`.
            FieldType::String | FieldType::Reference | FieldType::Identifier => {
                json!({"type": "string"})
            }
            FieldType::Enum if self.enum_values.is_empty() => json!({"type": "string"}),
            FieldType::Enum => json!({"type": "string", "enum": self.enum_values}),
            FieldType::Integer => {
                let mut schema = json!({"type": "integer"});
                if let Some(minimum) = self.bounds.0 {
                    schema["minimum"] = json!(minimum);
                }
                if let Some(maximum) = self.bounds.1 {
                    schema["maximum"] = json!(maximum);
                }
                schema
            }
            FieldType::Bool => json!({"type": "boolean"}),
            FieldType::Date => json!({"type": "string", "format": "date"}),
            FieldType::StringList | FieldType::ReferenceList => {
                json!({"type": "array", "items": {"type": "string"}})
            }
            FieldType::EnumList if self.enum_values.is_empty() => {
                json!({"type": "array", "items": {"type": "string"}, "uniqueItems": true})
            }
            FieldType::EnumList => json!({
                "type": "array",
                "items": {"type": "string", "enum": self.enum_values},
                "uniqueItems": true,
            }),
            FieldType::Block => self.block_schema(defs),
            FieldType::BlockList => json!({"type": "array", "items": self.block_schema(defs)}),
        }
    }
}

impl FieldForm {
    /// A block's schema: closed to its members when it types any, else a map of undeclared values.
    fn block_schema(&self, defs: &mut Definitions) -> Value {
        match self.members() {
            Some(members) if !members.is_empty() => members::object_schema(members, defs),
            _ => json!({"type": "object", "additionalProperties": FieldJson::schema(defs)}),
        }
    }
}

/// Every registered field's form, and the kinds that take any field: what
/// the graph build and every reader ask what a field's value is and
/// references (ADR 0093). Built once per registry build.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldForms {
    /// kind -> field -> form: looked up by `&str`, so a reading allocates nothing.
    forms: HashMap<String, HashMap<String, FieldForm>>,
    member_kinds: BTreeSet<String>,
    /// Every kind a loaded extension declares.
    kinds: BTreeSet<String>,
}

impl FieldForms {
    /// `forms` maps (kind, field) to the field's form; `member_kinds` are
    /// the kinds that declare `open_fields`; `kinds` are every kind a loaded
    /// extension declares (what makes a form's target [`Target::Loaded`]).
    pub fn new(
        forms: HashMap<(String, String), FieldForm>,
        member_kinds: BTreeSet<String>,
        kinds: BTreeSet<String>,
    ) -> Self {
        let mut nested: HashMap<String, HashMap<String, FieldForm>> = HashMap::new();
        for ((kind, field), form) in forms {
            nested.entry(kind).or_default().insert(field, form);
        }
        Self {
            forms: nested,
            member_kinds,
            kinds,
        }
    }

    /// How `kind`'s field `field` reads: its declared form, an undeclared
    /// field of a kind that takes any field, or no form at all.
    pub fn reading(&self, kind: &str, field: &str) -> Reading<'_> {
        if let Some(form) = self.forms.get(kind).and_then(|fields| fields.get(field)) {
            Reading::Declared(form)
        } else if self.member_kinds.contains(kind) {
            Reading::Member
        } else {
            Reading::Undeclared
        }
    }

    /// `entry`'s value as the graph build stores it on a block of `kind` (ADR 0093, ADR 0156 D1, ADR 0182 D1):
    /// coerced by its field's form when declared and not already in it, else the value as written (borrowed).
    /// Idempotent. What [`Reading::references`], the derived walk and every copy of an inherited value read.
    pub fn stored<'e>(&self, kind: &str, entry: &'e FieldEntry) -> Cow<'e, FieldValue> {
        let coerced = match self.reading(kind, entry.key.as_str()) {
            Reading::Declared(form) => form.coerce(&entry.value, entry.value_span.as_ref()),
            Reading::Member | Reading::Undeclared => None,
        };
        coerced.map_or(Cow::Borrowed(&entry.value), Cow::Owned)
    }

    /// No field is declared.
    pub fn is_empty(&self) -> bool {
        self.forms.is_empty()
    }

    /// Every declared (kind, field) and its form.
    pub fn iter(&self) -> impl Iterator<Item = ((&str, &str), &FieldForm)> {
        self.forms.iter().flat_map(|(kind, fields)| {
            fields
                .iter()
                .map(move |(field, form)| ((kind.as_str(), field.as_str()), form))
        })
    }

    /// Every kind a loaded extension declares; empty when none is loaded.
    pub fn kinds(&self) -> &BTreeSet<String> {
        &self.kinds
    }

    /// The kinds that take any field (`open_fields`).
    pub fn member_kinds(&self) -> &BTreeSet<String> {
        &self.member_kinds
    }
}

/// One id a field's value references and where it writes it: the ID's own bytes, inside the quotes of a quoted
/// value ([`spell::id_span`], ADR 0218); `None` for a value built without a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference<'v> {
    pub id: &'v str,
    pub span: Option<SourceSpan>,
}

/// How a (kind, field) reads: see [`FieldForms::reading`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading<'f> {
    /// The field is declared; its form decides.
    Declared(&'f FieldForm),
    /// An undeclared field of a kind that takes any field.
    Member,
    /// No form: a closed kind's unknown field, an unregistered kind, a
    /// project without extensions.
    Undeclared,
}

impl Reading<'_> {
    /// What `entry`'s value references, in written order, each with its
    /// span (ADR 0093). The one rule every reader asks, of an entry as the graph build stores it: a written entry is
    /// stored ([`FieldForms::stored`]), as a typed edit's referrer scan does:
    ///
    /// | reading | a single value | a list item |
    /// |---|---|---|
    /// | declared `reference` | the value, bare or quoted | none |
    /// | declared `reference_list` | none (the build stores it as a list) | every item |
    /// | declared `string_list` | none | a scheme ref ID |
    /// | other declared types, a member | none | none |
    /// | undeclared | none | a bare word, a number or a scheme ref ID |
    ///
    /// Each reference's span is where it writes its ID ([`spell::id_span`]): inside the quotes of a quoted value.
    pub fn references<'v>(&self, entry: &'v FieldEntry) -> Vec<Reference<'v>> {
        let item = |item: &'v ListItem| Reference {
            id: item.text.as_str(),
            span: Some(spell::id_span(item.written, &item.span, &item.text)),
        };
        match (self, &entry.value) {
            (Self::Declared(form), FieldValue::Identifier(id) | FieldValue::String(id))
                if form.field_type() == FieldType::Reference =>
            {
                let written = match entry.value {
                    FieldValue::String(_) => Written::Quoted,
                    _ => Written::Bare,
                };
                vec![Reference {
                    id: id.as_str(),
                    span: entry
                        .value_span
                        .as_ref()
                        .map(|span| spell::id_span(written, span, id)),
                }]
            }
            (Self::Declared(form), FieldValue::List(items)) => match form.field_type() {
                FieldType::ReferenceList => items.iter().map(item).collect(),
                FieldType::StringList => items
                    .iter()
                    .filter(|i| i.written == Written::SchemeRef)
                    .map(item)
                    .collect(),
                _ => Vec::new(),
            },
            (Self::Undeclared, FieldValue::List(items)) => items
                .iter()
                .filter(|i| {
                    matches!(
                        i.written,
                        Written::Bare | Written::SchemeRef | Written::Literal
                    )
                })
                .map(item)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether a name written as `written`, as a value of the field or as
    /// an item of its list, may name an entity: the editor's
    /// reference positions. Any name on a declared `reference` or
    /// `reference_list`; a scheme ref ID on a `string_list`; nothing on
    /// another declared type; a bare word or scheme ref ID on a member or
    /// an undeclared field (what its word names, as the graph reads it).
    pub fn may_name(&self, written: Written) -> bool {
        match self {
            Self::Declared(form) => match form.field_type() {
                FieldType::Reference | FieldType::ReferenceList => true,
                FieldType::StringList => written == Written::SchemeRef,
                _ => false,
            },
            Self::Member | Self::Undeclared => {
                matches!(written, Written::Bare | Written::SchemeRef)
            }
        }
    }

    /// Whether the field is a declared `reference`: its edges link after
    /// reference cycles are detected.
    pub fn is_single_reference(&self) -> bool {
        matches!(self, Self::Declared(form) if form.field_type() == FieldType::Reference)
    }
}

/// The one-item list a single written value is stored as on a
/// `string_list` / `reference_list` field; a list is stored as written.
fn coerce_list(value: &FieldValue, span: Option<&SourceSpan>) -> Option<FieldValue> {
    let (text, written) = match value {
        FieldValue::String(s) => (s.clone(), Written::Quoted),
        FieldValue::Identifier(id) => (id.clone(), Written::Bare),
        FieldValue::Date(s) => (s.clone(), Written::Literal),
        FieldValue::Integer(n) => (n.to_string(), Written::Literal),
        FieldValue::Boolean(b) => (b.to_string(), Written::Literal),
        _ => return None,
    };
    Some(FieldValue::List(vec![ListItem {
        text,
        span: span?.clone(),
        written,
    }]))
}

impl Graph {
    /// Store every node's field values as their forms coerce them. Idempotent,
    /// so rebuilds may re-run it on already coerced nodes.
    pub(crate) fn coerce_field_values(&mut self, forms: &FieldForms) {
        if forms.is_empty() {
            return;
        }
        for node in self.nodes_mut() {
            let kind = node.kind.raw.as_str();
            for entry in node.fields.entries_mut() {
                if let Cow::Owned(value) = forms.stored(kind, entry) {
                    entry.value = value;
                }
            }
        }
    }
}
