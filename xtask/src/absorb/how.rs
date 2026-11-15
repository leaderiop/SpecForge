//! The value transforms of the table's homes, each pure: an old value (as `field_form::write` gives it) in, the
//! typed edit's input out.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use std::collections::BTreeMap;

use super::table::{Compare, How};

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
        How::Options => {
            let options: Vec<BTreeMap<String, Input>> = items(value)
                .into_iter()
                .map(|name| BTreeMap::from([("name".to_string(), Input::One(name))]))
                .collect();
            (!options.is_empty()).then_some(Input::Blocks(options))
        }
        How::As | How::Refs | How::InvertOnto => None,
    }
}

/// The computed value the old `value` means under `compare`; none when no new value can equal it.
pub fn expected(compare: Compare, value: &Value) -> Option<Value> {
    match compare {
        Compare::Superseded => Some(json!(value.as_str() != Some("superseded"))),
    }
}

/// An input as JSON, for the ledger.
pub fn input_json(input: &Input) -> Value {
    serde_json::to_value(input).unwrap_or(Value::Null)
}
