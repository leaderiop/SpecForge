use serde::Deserialize;
use serde_json::Value;
use specforge_emitter::SchemaEdgeType;

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::{ErrorCode, ToolOutcome};

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    include_edges: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    include_validation_rules: Option<bool>,
}

/// `specforge.schema`: the GraphProtocolSchema a full export embeds.
/// `kind` keeps that kind and the edge types that can start or end at it;
/// `include_edges: false` drops `edge_types`; `include_validation_rules`
/// adds the rules the loaded extensions declare.
pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let kind_filter = args.kind.as_deref();
    let mut schema = crate::operations::project_schema(state);

    if let Some(kind) = kind_filter {
        schema.entity_kinds.retain(|entry| entry.name == kind);
        schema.edge_types.retain(|edge| touches(edge, kind));
    }

    let mut schema = match serde_json::to_value(&schema) {
        Ok(value) => value,
        Err(err) => {
            return ToolOutcome::error(
                ErrorCode::InternalError,
                format!("schema serialization failed: {err}"),
            );
        }
    };
    if !args.include_edges.unwrap_or(true)
        && let Some(object) = schema.as_object_mut()
    {
        object.remove("edge_types");
    }
    if args.include_validation_rules.unwrap_or(false) {
        // The rules each loaded extension declares, tagged with its name.
        let rules: Vec<Value> = state
            .registries()
            .declarations()
            .iter()
            .flat_map(|declaration| {
                declaration.validation_rules.iter().map(|rule| {
                    let mut rule = rule_json(rule);
                    rule["extension"] = Value::from(declaration.name());
                    rule
                })
            })
            .filter(|rule| kind_filter.is_none_or(|kind| rule["targetKind"].as_str() == Some(kind)))
            .collect();
        schema["validation_rules"] = Value::Array(rules);
    }

    ToolOutcome::ok(schema)
}

/// Whether an edge type can start or end at `kind`. An edge that names no
/// kinds on a side is open on that side.
fn touches(edge: &SchemaEdgeType, kind: &str) -> bool {
    let on =
        |kinds: &Option<Vec<String>>| kinds.as_ref().is_none_or(|k| k.iter().any(|k| k == kind));
    on(&edge.source_kinds) || on(&edge.target_kinds)
}

/// A declared validation rule as the schema lists it: its descriptor, keys
/// in camelCase (`message_template` is `messageTemplate`).
fn rule_json(rule: &specforge_protocol_types::ValidationRuleDescriptor) -> Value {
    let Ok(Value::Object(fields)) = serde_json::to_value(rule) else {
        unreachable!("a validation rule descriptor serializes to an object")
    };
    fields
        .into_iter()
        .map(|(key, value)| (camel_case(&key), value))
        .collect()
}

/// `snake_case` as `camelCase`.
fn camel_case(key: &str) -> String {
    let mut words = key.split('_');
    let mut out = words.next().unwrap_or_default().to_string();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_protocol_types::ValidationRuleDescriptor;

    #[test]
    fn a_rule_is_listed_with_camel_case_keys() {
        let rule = ValidationRuleDescriptor {
            code: "W901".to_string(),
            message_template: "{id} has no owner".to_string(),
            check: "field_required".to_string(),
            target_kind: Some("memo".to_string()),
            ..Default::default()
        };
        let json = rule_json(&rule);
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "check",
                "code",
                "constraint",
                "edgeType",
                "field",
                "messageTemplate",
                "severity",
                "targetKind",
                "wasmFunction"
            ]
        );
        assert_eq!(json["targetKind"], "memo");
        assert_eq!(json["messageTemplate"], "{id} has no owner");
        assert_eq!(camel_case("wasm_function"), "wasmFunction");
        assert_eq!(camel_case("code"), "code");
    }
}
