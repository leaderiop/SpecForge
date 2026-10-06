//! `specforge schema` and `specforge.schema`: the versioned Graph Protocol
//! schema, one operation over the project view (ADR 0015).

use serde_json::Value;
use specforge_emitter::{GraphProtocolSchema, SchemaEdgeType};

use crate::OpError;
use crate::view::ProjectView;

/// What part of the schema to return.
#[derive(Debug, Clone, Copy)]
pub struct SchemaRequest<'r> {
    /// That kind and the edge types that can start or end at it.
    pub kind: Option<&'r str>,
    /// Include the edge types.
    pub edges: bool,
    /// Add the validation rules the loaded extensions declare.
    pub validation_rules: bool,
}

impl Default for SchemaRequest<'_> {
    fn default() -> Self {
        SchemaRequest {
            kind: None,
            edges: true,
            validation_rules: false,
        }
    }
}

/// The schema a request selects.
#[derive(Debug, Clone)]
pub struct SchemaOutcome {
    /// Versioned as `specforge export` versions it; filtered by kind.
    pub schema: GraphProtocolSchema,
    /// Whether `schema.edge_types` is part of the answer.
    pub edges: bool,
    /// Each loaded extension's rules, tagged `extension`, when asked for.
    pub validation_rules: Option<Vec<Value>>,
}

impl SchemaOutcome {
    /// The schema as JSON: without `edge_types` when edges were not asked
    /// for, with `validation_rules` when they were.
    pub fn to_json(&self) -> Value {
        let mut doc = serde_json::to_value(&self.schema).expect("a schema serializes");
        if let Some(object) = doc.as_object_mut() {
            if !self.edges {
                object.remove("edge_types");
            }
            if let Some(rules) = &self.validation_rules {
                object.insert("validation_rules".into(), Value::Array(rules.clone()));
            }
        }
        doc
    }
}

/// The view's versioned schema, as `request` selects it. A kind no loaded
/// extension declares is `unknown_kind`, naming the closest one.
pub fn schema(view: &ProjectView, request: &SchemaRequest) -> Result<SchemaOutcome, OpError> {
    let mut schema = view.versioned_schema();
    if let Some(kind) = request.kind {
        if !schema.entity_kinds.iter().any(|entry| entry.name == kind) {
            return Err(unknown_kind(kind, &schema));
        }
        schema.entity_kinds.retain(|entry| entry.name == kind);
        schema.edge_types.retain(|edge| touches(edge, kind));
    }
    let validation_rules = request.validation_rules.then(|| {
        view.registries
            .declarations()
            .iter()
            .flat_map(|declaration| {
                declaration.validation_rules.iter().map(|rule| {
                    let mut rule = rule_json(rule);
                    rule["extension"] = Value::from(declaration.name());
                    rule
                })
            })
            .filter(|rule| {
                request
                    .kind
                    .is_none_or(|kind| rule["targetKind"].as_str() == Some(kind))
            })
            .collect()
    });
    Ok(SchemaOutcome {
        schema,
        edges: request.edges,
        validation_rules,
    })
}

fn unknown_kind(kind: &str, schema: &GraphProtocolSchema) -> OpError {
    let error = OpError::new("unknown_kind", format!("unknown entity kind: '{kind}'"));
    let known = schema.entity_kinds.iter().map(|entry| entry.name.as_str());
    match specforge_common::suggest::find_close_match(kind, known) {
        Some(close) => error.with_suggestion(format!("did you mean '{close}'?")),
        None => error,
    }
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
