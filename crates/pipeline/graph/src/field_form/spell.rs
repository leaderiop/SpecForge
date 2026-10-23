//! How a value is spelled in `.spec` text (ADR 0147, ADR 0153): the one escape table ([`escaped`], [`quoted`]), the
//! one rule for when a value may stand bare, and each declared type's one-line spelling. `FieldForms::rewrite`
//! (format's rewrite and W159), the typed-edit writer and create, migrate's `titles` step, W159's message and init's
//! starter read it; nothing else quotes a value. [`form`] is the one table that says how a declared type writes a
//! text. It also says where a quoted value writes the id it reads
//! ([`id_span`]).

use specforge_common::SourceSpan;
use specforge_parser::lex::is_entity_id;
use specforge_parser::{FieldEntry, FieldValue, Written};
use specforge_protocol_types::FieldType;

use super::{FieldForm, FieldForms, Reading, is_word};

/// `text` as the inside of a one-line `"…"`: `\`, `"`, line feed, tab and carriage return escaped, every other
/// character as it is. What a template writes between quotes; [`quoted`] is this between two `"`.
pub fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

/// `text` as a one-line `"…"`: [`escaped`] between quotes. The parser's string reading gives `text` back.
pub fn quoted(text: &str) -> String {
    format!("\"{}\"", escaped(text))
}

/// Where a value written `written` at `span` writes the id `id` it reads (ADR 0218): one byte inside each quote of a
/// `"…"` on one line exactly two bytes wider than `id`, three inside each of a `"""…"""` exactly six wider when `id`
/// is an entity id; else `span` (a bare word, a scheme ref or a literal is its text; a value with an escape, padding
/// or a line break has no bytes that spell its id).
///
/// Exact, not a guess: a `"…"` reads each escape (see [`quoted`]) as one character and keeps any other byte, so
/// content as long as `id` holds no escape and is `id`. An entity id holds none of the escaped characters, so a value
/// six bytes wider than one is `"""id"""`.
pub fn id_span(written: Written, span: &SourceSpan, id: &str) -> SourceSpan {
    let width = span.end_col.checked_sub(span.start_col);
    let one_line = span.start_line == span.end_line;
    let quote = match (written, width) {
        (Written::Quoted, Some(w)) if one_line && w == id.len() + 2 => 1,
        (Written::Quoted, Some(w)) if one_line && w == id.len() + 6 && is_entity_id(id) => 3,
        _ => return span.clone(),
    };
    SourceSpan {
        start_col: span.start_col + quote,
        end_col: span.end_col - quote,
        ..span.clone()
    }
}

/// `text` written as `written`: `Quoted` through [`quoted`]; bare, a scheme ref or a literal as it is.
pub fn spelled(text: &str, written: Written) -> String {
    match written {
        Written::Quoted => quoted(text),
        Written::Bare | Written::SchemeRef | Written::Literal => text.to_string(),
    }
}

/// Whether `text` may stand bare as a value of a field of `field_type` whose form is bare (ADR 0147 D1): a word,
/// or on a `reference_list` an entity id (a scheme ref id too, ADR 0153 D3); never `true` / `false` (they read back
/// as booleans). A scalar `reference` takes a word only: a scheme ref id does not parse bare outside a list.
pub fn holds_bare(field_type: FieldType, text: &str) -> bool {
    !matches!(text, "true" | "false")
        && match field_type {
            FieldType::ReferenceList => is_entity_id(text),
            _ => is_word(text),
        }
}

/// Whether `text` is one calendar day written `YYYY-MM-DD` (ISO 8601, the proleptic Gregorian calendar): the one
/// text a `date` field holds and a date literal stands bare as (ADR 0234). `2026-02-29` is none; `2024-02-29` is one.
pub fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !digits(0..4)
        || !digits(5..7)
        || !digits(8..10)
    {
        return false;
    }
    let number = |r: std::ops::Range<usize>| text[r].parse::<u32>().unwrap_or(0);
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    (1..=last).contains(&day)
}

/// How a field of `form` writes `text`, one value or one item of its list: the one table (ADR 0234).
///
/// | declared | written |
/// |---|---|
/// | `integer` | a literal when `text` is an integer; else quoted (E061 reports it) |
/// | `bool` | a literal when `true` / `false`; else quoted (E061) |
/// | `date` | a literal when [`is_date`]; else quoted (E061) |
/// | `string_list` item | a scheme ref id as a scheme ref (it then names an entity, ADR 0093); else quoted |
/// | `reference_list` item | an entity id bare ([`holds_bare`]: a word or a scheme ref id), never `true` / `false`; else quoted |
/// | `reference`, `enum`, `identifier` | bare when [`holds_bare`] (a word); else quoted |
/// | `string`, `block` | quoted |
pub fn form(form: &FieldForm, text: &str) -> Written {
    let field_type = form.field_type();
    match field_type {
        FieldType::Integer if text.trim().parse::<i64>().is_ok() => Written::Literal,
        FieldType::Bool if matches!(text, "true" | "false") => Written::Literal,
        FieldType::Date if is_date(text) => Written::Literal,
        FieldType::StringList if text.contains(':') && is_entity_id(text) => Written::SchemeRef,
        FieldType::Enum
        | FieldType::Identifier
        | FieldType::Reference
        | FieldType::ReferenceList
            if holds_bare(field_type, text) =>
        {
            Written::Bare
        }
        _ => Written::Quoted,
    }
}

/// `text`, one value of a field of `form` or one item of its list, written on one line in the form [`form`] says
/// (an integer as the number it parses to).
pub fn value(form: &FieldForm, text: &str) -> String {
    match (form.field_type(), text.trim().parse::<i64>()) {
        (FieldType::Integer, Ok(n)) => n.to_string(),
        _ => spelled(text, self::form(form, text)),
    }
}

/// Where a value stands in its field: the field's whole value, or an item of a list written `[…]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Whole,
    Item,
}

/// What a value written other than its field's form becomes: the form its text takes ([`form`]) and, for a whole
/// value on a `string_list` / `reference_list` field, that it becomes the one-item list `[…]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Respell {
    pub to: Written,
    pub listed: bool,
}

/// A value written in other than its field's form: where it is (quotes included), its text, the form it takes, and
/// the whole entry's value as the field writes it on one line (what W159's message prints).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewrite {
    pub span: SourceSpan,
    pub text: String,
    pub to: Written,
    pub listed: bool,
    pub value: String,
}

impl FieldForms {
    /// How `kind.field` writes `text`, written `written` at `place`, when that is not how it is written (ADR 0147,
    /// ADR 0234); `None` leaves it as written: the field has no declared form or is a `block`; the value is already
    /// in its form (quoted exactly when the form is quoted, and listed or not as the type says); a number, boolean
    /// or date field holds a text that is none (E061 reports it); a quoted scheme ref id on a `string_list`
    /// (unquoted it would name an entity, ADR 0093); a bare word on a kind that takes members, when its form is
    /// quoted (it may be a member's type).
    pub fn rewrite(
        &self,
        kind: &str,
        field: &str,
        written: Written,
        text: &str,
        place: Place,
    ) -> Option<Respell> {
        let Reading::Declared(form) = self.reading(kind, field) else {
            return None;
        };
        let field_type = form.field_type();
        if field_type == FieldType::Block {
            return None;
        }
        let listed = place == Place::Whole
            && matches!(field_type, FieldType::StringList | FieldType::ReferenceList);
        let to = match self::form(form, text) {
            Written::SchemeRef if written == Written::Quoted => Written::Quoted,
            to => to,
        };
        let refused = matches!(
            field_type,
            FieldType::Integer | FieldType::Bool | FieldType::Date
        ) && to == Written::Quoted;
        let same = (written == Written::Quoted) == (to == Written::Quoted);
        if refused || (same && !listed) {
            return None;
        }
        if written == Written::Bare && to == Written::Quoted && self.member_kinds.contains(kind) {
            return None;
        }
        Some(Respell { to, listed })
    }

    /// Every value of `entry` (a scalar, or each list item) that
    /// [`Self::rewrite`] would write otherwise, in source order. A scalar
    /// without a value span is reported at `fallback`.
    pub fn rewrites(&self, kind: &str, entry: &FieldEntry, fallback: &SourceSpan) -> Vec<Rewrite> {
        let field = entry.key.as_str();
        let span = entry.value_span.as_ref().unwrap_or(fallback);
        let scalar = |written, text: String| {
            let respell = self.rewrite(kind, field, written, &text, Place::Whole)?;
            let one = spelled(&text, respell.to);
            let value = match (respell.listed, text.contains('\n')) {
                (false, _) => one,
                (true, false) => format!("[{one}]"),
                (true, true) => {
                    let lines: Vec<String> = text
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(quoted)
                        .collect();
                    format!("[{}]", lines.join(", "))
                }
            };
            Some(Rewrite {
                span: span.clone(),
                text,
                to: respell.to,
                listed: respell.listed,
                value,
            })
        };
        match &entry.value {
            FieldValue::String(text) => scalar(Written::Quoted, text.clone()).into_iter().collect(),
            FieldValue::Identifier(text) => {
                scalar(Written::Bare, text.clone()).into_iter().collect()
            }
            FieldValue::Integer(n) => scalar(Written::Literal, n.to_string())
                .into_iter()
                .collect(),
            FieldValue::Boolean(b) => scalar(Written::Literal, b.to_string())
                .into_iter()
                .collect(),
            FieldValue::Date(text) => scalar(Written::Literal, text.clone()).into_iter().collect(),
            FieldValue::List(items) => {
                let respelled: Vec<Option<Respell>> = items
                    .iter()
                    .map(|i| self.rewrite(kind, field, i.written, &i.text, Place::Item))
                    .collect();
                let value = format!(
                    "[{}]",
                    items
                        .iter()
                        .zip(&respelled)
                        .map(|(i, r)| spelled(&i.text, r.map_or(i.written, |r| r.to)))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                items
                    .iter()
                    .zip(respelled)
                    .filter_map(|(i, r)| {
                        r.map(|r| Rewrite {
                            span: i.span.clone(),
                            text: i.text.clone(),
                            to: r.to,
                            listed: false,
                            value: value.clone(),
                        })
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}
