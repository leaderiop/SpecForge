use std::path::Path;

use specforge_emitter::generate_schema;
use specforge_emitter::model::{
    FieldLevel as EmitterFieldLevel, GroupBy as EmitterGroupBy, ModelFormat as EmitterModelFormat,
    ModelIntermediate_from_schema, ModelOptions, filter_entities, filter_fields, render,
};

use crate::pipeline;
use crate::{FieldLevel, GroupBy, ModelFormat};

#[allow(clippy::too_many_arguments)]
pub fn run(
    path: &Path,
    format: ModelFormat,
    group_by: GroupBy,
    fields: FieldLevel,
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
        ModelFormat::Markdown => EmitterModelFormat::Markdown,
        ModelFormat::Mermaid => EmitterModelFormat::Mermaid,
        ModelFormat::Dot => EmitterModelFormat::Dot,
        ModelFormat::Json => EmitterModelFormat::Json,
        ModelFormat::Dbml => EmitterModelFormat::Dbml,
    };

    let group = match group_by {
        GroupBy::Extension => EmitterGroupBy::Extension,
        GroupBy::None => EmitterGroupBy::None,
    };

    let field_level = match fields {
        FieldLevel::None => EmitterFieldLevel::None,
        FieldLevel::Keys => EmitterFieldLevel::Keys,
        FieldLevel::All => EmitterFieldLevel::All,
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

    let model = ModelIntermediate_from_schema(&schema).with_theme_colors(&ctx.manifests);
    for warning in &model.warnings {
        eprintln!("warning (model): {warning}");
    }
    let model = filter_entities(&model, &options);
    let model = filter_fields(&model, options.fields);

    let output = render(&model, &options);
    print!("{}", output);

    0
}
