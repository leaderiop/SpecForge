//! What every core tool replies today, defects included (plan 11).
//!
//! These pins are unlinked: they hold the replies as they are, `null`s and
//! undeclared keys among them. The ticket that changes what a pin holds
//! re-blesses its snapshot in the same commit, so the diff is the
//! user-visible change.

use serde_json::Value;

use crate::support::replies::{Observed, listed_output_schemas, read_all, undeclared, write_all};

/// An object as `{k: shape, …}` with sorted keys, an array as
/// `[shape | shape …]` (the distinct element shapes, sorted), a scalar as
/// its JSON type.
fn shape(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "boolean".into(),
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer".into(),
        Value::Number(_) => "number".into(),
        Value::String(_) => "string".into(),
        Value::Array(items) => {
            let mut shapes: Vec<String> = items.iter().map(shape).collect();
            shapes.sort();
            shapes.dedup();
            format!("[{}]", shapes.join(" | "))
        }
        Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let fields: Vec<String> = keys
                .into_iter()
                .map(|key| format!("{key}: {}", shape(&object[key])))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
    }
}

fn observed() -> Vec<Observed> {
    let mut all = read_all();
    all.extend(write_all());
    all
}

#[test]
fn reply_shapes_today() {
    let lines: Vec<String> = observed()
        .iter()
        .map(|call| {
            format!(
                "{} {} => {}",
                call.tool,
                call.arguments,
                shape(&call.reply())
            )
        })
        .collect();
    insta::assert_snapshot!(lines.join("\n"));
}

#[test]
fn undeclared_reply_keys_today() {
    let listing = listed_output_schemas();
    let mut lines = Vec::new();
    for call in observed() {
        let Some(structured) = call.response["result"].get("structuredContent") else {
            continue;
        };
        let Some(schema) = listing.get(call.tool) else {
            continue;
        };
        for path in undeclared(schema, structured) {
            lines.push(format!("{} {} {path}", call.tool, call.arguments));
        }
    }
    insta::assert_snapshot!(lines.join("\n"));
}
