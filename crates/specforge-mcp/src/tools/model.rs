use serde::Deserialize;
use specforge_emitter::generate_schema;
use specforge_emitter::model::{
    FieldLevel, GroupBy, ModelFormat, ModelIntermediate_from_schema, ModelOptions, filter_entities,
    filter_fields, render,
};

use crate::args::{lenient, some_strings};
use crate::target::Call;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    format: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    group_by: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    fields: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    extension: Option<String>,
    #[serde(default, deserialize_with = "some_strings")]
    kinds: Option<Vec<String>>,
    #[serde(default, deserialize_with = "lenient")]
    root: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    depth: Option<u64>,
}

pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let state = &*call.state;
    let format = args.format.as_deref().unwrap_or("markdown");
    let group_by = args.group_by.as_deref().unwrap_or("extension");
    let fields = args.fields.as_deref().unwrap_or("keys");
    let extension = args.extension.as_deref();
    let root = args.root.as_deref();
    let depth = args.depth.map(|d| d as usize);

    let model_format = match format {
        "markdown" => ModelFormat::Markdown,
        "mermaid" => ModelFormat::Mermaid,
        "dot" => ModelFormat::Dot,
        "json" => ModelFormat::Json,
        "dbml" => ModelFormat::Dbml,
        _ => {
            return ToolOutcome::invalid_input(
                "format",
                format!(
                    "Unknown format: {}. Expected: markdown, mermaid, dot, json, dbml",
                    format
                ),
            );
        }
    };

    let group = match group_by {
        "extension" => GroupBy::Extension,
        "none" => GroupBy::None,
        _ => {
            return ToolOutcome::invalid_input(
                "group_by",
                format!("Unknown group_by: {}. Expected: extension, none", group_by),
            );
        }
    };

    let field_level = match fields {
        "none" => FieldLevel::None,
        "keys" => FieldLevel::Keys,
        "all" => FieldLevel::All,
        _ => {
            return ToolOutcome::invalid_input(
                "fields",
                format!("Unknown fields: {}. Expected: none, keys, all", fields),
            );
        }
    };

    let kind_filter = args.kinds.clone();

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
        &state.registries().kinds,
        &state.registries().edges,
        &state.registries().fields,
        &state.registries().extension_info,
    );

    let model =
        ModelIntermediate_from_schema(&schema).with_theme_colors(&state.registries().manifests);
    let model = filter_entities(&model, &options);
    let model = filter_fields(&model, options.fields);
    let output = render(&model, &options);

    ToolOutcome::text(output)
}
