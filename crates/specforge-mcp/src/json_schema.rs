//! Checking a JSON value against a tool's JSON Schema: the subset tool
//! schemas use (`type`, `enum`, `properties`, `required`, `items`, `oneOf`,
//! `anyOf`). Other keywords are not checked.

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
    for (keyword, exactly_one) in [("oneOf", true), ("anyOf", false)] {
        let Some(Value::Array(branches)) = schema.get(keyword) else {
            continue;
        };
        let matching = branches
            .iter()
            .filter(|branch| {
                let mut branch_found = Vec::new();
                check(branch, value, path, &mut branch_found);
                branch_found.is_empty()
            })
            .count();
        let conforms = if exactly_one {
            matching == 1
        } else {
            matching > 0
        };
        if !conforms {
            found.push(format!(
                "{path}: matches {matching} of the {} {keyword} schemas",
                branches.len()
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::violations;
    use serde_json::json;

    #[test]
    fn one_of_needs_exactly_one_branch_and_any_of_at_least_one() {
        let branch = |key: &str| json!({"type": "object", "required": [key]});
        let one_of = json!({"type": "object", "oneOf": [branch("a"), branch("b")]});
        assert!(violations(&one_of, &json!({"a": 1})).is_empty());
        assert_eq!(
            violations(&one_of, &json!({"c": 1})),
            ["$: matches 0 of the 2 oneOf schemas"]
        );
        assert_eq!(
            violations(&one_of, &json!({"a": 1, "b": 2})),
            ["$: matches 2 of the 2 oneOf schemas"]
        );
        let any_of = json!({"anyOf": [branch("a"), branch("b")]});
        assert!(violations(&any_of, &json!({"a": 1, "b": 2})).is_empty());
        assert_eq!(
            violations(&any_of, &json!({})),
            ["$: matches 0 of the 2 anyOf schemas"]
        );
    }
}
