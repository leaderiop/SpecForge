use std::path::Path;

use specforge_emitter::generate_schema;
use specforge_emitter::model::{
    filter_entities, filter_fields, render, FieldLevel, GroupBy, ModelFormat,
    ModelIntermediate_from_schema, ModelOptions,
};

use crate::pipeline;

pub fn run(
    path: &Path,
    format: &str,
    group_by: &str,
    fields: &str,
    extension: Option<&str>,
    kinds: &[String],
    root: Option<&str>,
    depth: Option<usize>,
) -> i32 {
    let ctx = pipeline::compile(path);

    let schema = generate_schema(
        &ctx.kind_registry,
        &ctx.edge_registry,
        &ctx.field_registry,
        &ctx.extension_info,
    );

    let model_format = match format {
        "mermaid" => ModelFormat::Mermaid,
        "dot" => ModelFormat::Dot,
        "json" => ModelFormat::Json,
        "dbml" => ModelFormat::Dbml,
        _ => ModelFormat::Markdown,
    };

    let group = match group_by {
        "none" => GroupBy::None,
        _ => GroupBy::Extension,
    };

    let field_level = match fields {
        "none" => FieldLevel::None,
        "all" => FieldLevel::All,
        _ => FieldLevel::Keys,
    };

    let kind_filter = if kinds.is_empty() {
        None
    } else {
        Some(kinds.to_vec())
    };

    let options = ModelOptions {
        format: model_format,
        group_by: group,
        fields: field_level,
        extension_filter: extension.map(|s| s.to_string()),
        kind_filter,
        root: root.map(|s| s.to_string()),
        depth,
    };

    let model = ModelIntermediate_from_schema(&schema);
    let model = filter_entities(&model, &options);
    let model = filter_fields(&model, options.fields);

    let output = render(&model, &options);
    print!("{}", output);

    0
}
