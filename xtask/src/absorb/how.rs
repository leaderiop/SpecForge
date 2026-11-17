//! The value transforms of the table's homes, each pure: an old value (as `field_form::write` gives it) in, the
//! typed edit's input out.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use specforge_parser::lex::is_entity_id;

use super::table::{Compare, How};

/// Why a scheme reference in a list of texts is not written: the writer spells it bare, which names a `ref`.
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

/// The input a word-mapping `how` writes for `value`; none when the word maps to nothing.
pub fn word(how: How, value: &Value) -> Option<Input> {
    let word = value.as_str().unwrap_or_default();
    match how {
        How::Outcome => match word {
            "proposed" => Some(Input::One("pending".to_string())),
            "deprecated" => Some(Input::One("revisit".to_string())),
            _ => None,
        },
        How::Settled => (word == "accepted").then_some(Input::Bool(true)),
        _ => None,
    }
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

/// A phase's name from its old milestone's title: the title less a leading `Phase `, its digits, an optional letter,
/// `:` and the spaces after it (GSD renders the number from the phase's id), else the title as written.
pub fn phase_name(title: &str) -> String {
    numbered(title).unwrap_or(title).to_string()
}

/// What follows a leading `Phase <digits><letter?>: ` in `title`, when it does and something follows.
fn numbered(title: &str) -> Option<&str> {
    let rest = title.strip_prefix("Phase ")?;
    let after = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    if after.len() == rest.len() {
        return None;
    }
    let after = after
        .strip_prefix(|c: char| c.is_ascii_alphabetic())
        .filter(|tail| tail.starts_with(':'))
        .unwrap_or(after);
    let name = after.strip_prefix(':')?.trim_start();
    (!name.is_empty()).then_some(name)
}
