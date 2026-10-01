//! Checking a JSON value against a tool's JSON Schema: the subset tool
//! schemas use (`type`, `enum`, `properties`, `required`, `items`). Other
//! keywords are not checked.

use serde_json::Value;

/// Where `value` breaks `schema`, one message per violation, each naming
/// its path (`$.format`, `$.paths[0]`). Empty when it conforms.
pub fn violations(schema: &Value, value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    check(schema, value, "$", &mut found);
    found
}

fn check(schema: &Value, value: &Value, path: &str, found: &mut Vec<String>) {
    let types: Vec<&str> = match schema.get("type") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    if !types.is_empty() && !types.iter().any(|name| is_a(name, value)) {
        found.push(format!(
            "{path}: expected {}, got {}",
            types.join(" or "),
            type_name(value)
        ));
        return;
    }
    if let Some(Value::Array(allowed)) = schema.get("enum")
        && !allowed.contains(value)
    {
        found.push(format!(
            "{path}: {value} is not one of {}",
            Value::from(allowed.clone())
        ));
    }
    if let Value::Object(object) = value {
        for name in schema["required"].as_array().into_iter().flatten() {
            if let Some(name) = name.as_str()
                && !object.contains_key(name)
            {
                found.push(format!("{path}: missing required {name}"));
            }
        }
        if let Some(properties) = schema["properties"].as_object() {
            for (name, property) in properties {
                if let Some(member) = object.get(name) {
                    check(property, member, &format!("{path}.{name}"), found);
                }
            }
        }
    }
    if let (Value::Array(items), Some(item)) = (value, schema.get("items")) {
        for (i, element) in items.iter().enumerate() {
            check(item, element, &format!("{path}[{i}]"), found);
        }
    }
}

/// Whether `value` is of the JSON Schema type `name`; a name the subset
/// does not know accepts anything.
fn is_a(name: &str, value: &Value) -> bool {
    match name {
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_f64() => "number",
        Value::Number(_) => "integer",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
