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
            .manifests
            .iter()
            .flat_map(|manifest| {
                manifest.validation_rules.iter().filter_map(|rule| {
                    let mut rule = serde_json::to_value(rule).ok()?;
                    rule["extension"] = Value::from(manifest.name.as_str());
                    Some(rule)
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
