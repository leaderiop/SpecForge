use serde::Deserialize;
use serde_json::Value;
use specforge_emitter::SchemaEdgeType;

use crate::args::lenient;
use crate::target::Call;
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
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let state = &*call.state;
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
            .manifests
            .iter()
            .flat_map(|manifest| {
                manifest.validation_rules.iter().filter_map(|rule| {
                    let mut rule = serde_json::to_value(rule).ok()?;
                    rule["extension"] = Value::from(manifest.name.as_str());
                    Some(rule)
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
