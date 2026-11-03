//! The canonical form of a request or an outcome, and its digest: what a journal keys a record by and a verdict
//! compares (ADR 0335 D3). Plan 03's effect journal is their caller.

use serde::Serialize;
use serde_json::{Map, Value};
use specforge_package::Sha256;
use specforge_protocol_types::REDACTED;

/// `value` as JSON with every object's keys sorted, and the `value` of every object marked `"sensitive": true` (a
/// credential header) written [`REDACTED`]. Strings are kept byte for byte: a `\r\n` is not a `\n` here.
pub fn canonical(value: &impl Serialize) -> Vec<u8> {
    let value = serde_json::to_value(value).expect("an effect's wire type has string keys only");
    serde_json::to_vec(&sorted(value)).expect("a JSON value serializes")
}

/// `sha256:` and the hex digest of [`canonical`]`(value)`.
pub fn digest(value: &impl Serialize) -> String {
    format!("sha256:{}", Sha256::of(&canonical(value)))
}

fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let redact = map.get("sensitive") == Some(&Value::Bool(true));
            let mut entries: Vec<(String, Value)> = map
                .into_iter()
                .map(|(key, value)| (key, sorted(value)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            if redact {
                for (key, value) in &mut entries {
                    if key == "value" {
                        *value = Value::String(REDACTED.to_string());
                    }
                }
            }
            Value::Object(entries.into_iter().collect::<Map<_, _>>())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}
