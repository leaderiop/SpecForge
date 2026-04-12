use serde_json::Value;
use specforge_emitter::generate_schema;
use specforge_emitter::model::{
    filter_entities, filter_fields, render, FieldLevel, GroupBy, ModelFormat,
    ModelIntermediate_from_schema, ModelOptions,
};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("markdown");
    let group_by = args.get("group_by").and_then(|v| v.as_str()).unwrap_or("extension");
    let fields = args.get("fields").and_then(|v| v.as_str()).unwrap_or("keys");
    let extension = args.get("extension").and_then(|v| v.as_str());
    let root = args.get("root").and_then(|v| v.as_str());
    let depth = args.get("depth").and_then(|v| v.as_u64()).map(|d| d as usize);

    let model_format = match format {
        "markdown" => ModelFormat::Markdown,
        "mermaid" => ModelFormat::Mermaid,
        "dot" => ModelFormat::Dot,
        "json" => ModelFormat::Json,
        "dbml" => ModelFormat::Dbml,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown format: {}. Expected: markdown, mermaid, dot, json, dbml", format),
        ),
    };

    let group = match group_by {
        "extension" => GroupBy::Extension,
        "none" => GroupBy::None,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown group_by: {}. Expected: extension, none", group_by),
        ),
    };

    let field_level = match fields {
        "none" => FieldLevel::None,
        "keys" => FieldLevel::Keys,
        "all" => FieldLevel::All,
        _ => return JsonRpcResponse::error(
            id, error_codes::INVALID_PARAMS,
            format!("Unknown fields: {}. Expected: none, keys, all", fields),
        ),
    };

    let kind_filter = args.get("kinds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let options = ModelOptions {
        format: model_format,
        group_by: group,
        fields: field_level,
        extension_filter: extension.map(String::from),
        kind_filter,
        root: root.map(String::from),
        depth,
    };

    let schema = generate_schema(
        &state.kind_registry,
        &state.edge_registry,
        &state.field_registry,
        &state.extension_info,
    );

    let model = ModelIntermediate_from_schema(&schema);
    let model = filter_entities(&model, &options);
    let model = filter_fields(&model, options.fields);
    let output = render(&model, &options);

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": output
        }]
    }))
}
