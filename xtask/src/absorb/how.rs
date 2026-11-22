//! The value transforms of the table's homes, each pure: an old value (as `field_form::write` gives it) in, the
//! typed edit's input out.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use specforge_parser::lex::is_entity_id;

use super::table::Compare;

/// Why a scheme reference in a list of texts is dropped (ADR 0372 D8): the writer spells it bare, which names a `ref`.
pub const SCHEME_REF: &str =
    "a scheme reference is written bare and names a `ref`, which a typed edit cannot create";

/// `value` as written: a text or a date as one value, a list as its items, a number or a boolean as itself.
pub fn as_written(value: &Value) -> Input {
    match value {
        Value::String(text) => Input::One(text.clone()),
        Value::Array(_) => Input::Many(items(value)),
        Value::Number(n) => Input::Number(n.clone()),
        Value::Bool(b) => Input::Bool(*b),
        other => Input::One(other.to_string()),
    }
}

/// The items of a list, or a single value as one item.
pub fn items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(text).collect(),
        Value::Null => Vec::new(),
        other => vec![text(other)],
    }
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Whether a list item is a scheme reference id (`gh.issue:42`): see [`SCHEME_REF`].
pub fn is_scheme_ref(item: &str) -> bool {
    item.contains(':') && is_entity_id(item)
}

/// The `settled` a decision status writes: true for `accepted`; none for any other word.
pub fn settled(value: &Value) -> Option<Input> {
    (value.as_str() == Some("accepted")).then_some(Input::Bool(true))
}

/// What a computed value is checked against.
#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    /// The value the old one means.
    Value(Value),
    /// No new value equals it: always ledgered with the new one.
    Never,
}

/// The computed value the old `value` means under `compare`.
pub fn expected(compare: Compare, value: &Value) -> Expect {
    let Compare::Means(words) = compare;
    words
        .iter()
        .find(|(old, _)| value.as_str() == Some(old))
        .map_or(Expect::Never, |(_, new)| Expect::Value(json!(new)))
}

/// The tier a feature status gives its requirement.
pub fn feature_tier(status: &str) -> &'static str {
    match status {
        "proposed" | "deferred" => "v2",
        "deprecated" => "out_of_scope",
        _ => "v1",
    }
}

/// The old default status of a feature, which a feature that writes none has.
pub const FEATURE_DEFAULT: &str = "proposed";

/// An input as JSON, for the ledger.
pub fn input_json(input: &Input) -> Value {
    serde_json::to_value(input).unwrap_or(Value::Null)
}

/// `text` as a slug: lower case, runs of other characters as one dash. Copied from `gsd/core/src/branch.rs` `slug`,
/// since no crate outside `gsd/` depends on the gsd crates: a phase's directory is named from it.
pub fn slug(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

/// The phase number `title` begins with and what follows it, separators dropped, when it begins with one: `Phase`
/// (any case) and a number in GSD's grammar (`\d+[A-Z]?(\.\d+)*`), then no letter or digit; or a bare number of
/// digits and dots, then `:` or `.` before a space or the end, `)`, or a dash between spaces (`03: Core`, `3. Core`,
/// `2) Core`, `04 - Core`). A number inside the title (`HTTP 2 support`, `2FA login`, `v2.0 migration`) is none.
///
/// Copied from `gsd/core/src/passes/roadmap/titles.rs` `numbered`, as [`slug`] is, since no crate outside `gsd/`
/// depends on the gsd crates: a converted phase's title is its name alone (ADR 0387), so the converter drops exactly
/// the number `gsd/W109` would flag.
pub fn numbered(title: &str) -> Option<(&str, &str)> {
    let text = title.trim_start();
    let end = match text.get(..5) {
        Some(word) if word.eq_ignore_ascii_case("phase") => worded(&text[5..]).map(|n| 5 + n)?,
        _ => bare(text)?,
    };
    let rest = text[end..].trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, ':' | '.' | ')' | '-' | '–' | '—')
    });
    Some((text[..end].trim_end(), rest.trim_end()))
}

/// The length of `rest` up to the end of the number after `Phase`: spaces, then a number in GSD's grammar, not
/// followed by a letter or a digit.
fn worded(rest: &str) -> Option<usize> {
    let spaces = rest.len() - rest.trim_start().len();
    let number = grammar(&rest[spaces..], true)?;
    let end = spaces + number;
    match rest[end..].chars().next() {
        Some(c) if c.is_alphanumeric() => None,
        _ => Some(end),
    }
}

/// The length of a bare number at the start of `text` when a label's separator follows it.
fn bare(text: &str) -> Option<usize> {
    let end = grammar(text, false)?;
    let after = &text[end..];
    let spaced = |tail: &str| tail.is_empty() || tail.starts_with(char::is_whitespace);
    let separated = match after.chars().next()? {
        ':' | '.' => spaced(&after[1..]),
        ')' => true,
        c if c.is_whitespace() => {
            let dash = after.trim_start();
            dash.starts_with(['-', '–', '—']) && spaced(&dash[dash.chars().next()?.len_utf8()..])
        }
        _ => false,
    };
    separated.then_some(end)
}

/// The length of the number at the start of `text`: digits, an optional capital letter when `lettered`, then any
/// `.digits`.
fn grammar(text: &str, lettered: bool) -> Option<usize> {
    let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let mut end = digits(text);
    if end == 0 {
        return None;
    }
    if lettered && text[end..].starts_with(|c: char| c.is_ascii_uppercase()) {
        end += 1;
    }
    while let Some(tail) = text[end..].strip_prefix('.') {
        match digits(tail) {
            0 => break,
            n => end += 1 + n,
        }
    }
    Some(end)
}
