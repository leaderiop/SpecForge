use std::path::Path;

use specforge_emitter::model::{
    FieldLevel as EmitterFieldLevel, GroupBy as EmitterGroupBy, ModelFormat as EmitterModelFormat,
    ModelOptions,
};
use specforge_ops::view::ProjectView;

use crate::pipeline;
use crate::{FieldLevel, GroupBy, ModelFormat};

/// `specforge model`: the model operation over the project compiled at
/// `path`; its W146 warnings go to stderr.
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
    let (project, _runtime) = pipeline::compile_project(path);
    let options = ModelOptions {
        format: match format {
            ModelFormat::Markdown => EmitterModelFormat::Markdown,
            ModelFormat::Mermaid => EmitterModelFormat::Mermaid,
            ModelFormat::Dot => EmitterModelFormat::Dot,
            ModelFormat::Json => EmitterModelFormat::Json,
            ModelFormat::Dbml => EmitterModelFormat::Dbml,
        },
        group_by: match group_by {
            GroupBy::Extension => EmitterGroupBy::Extension,
            GroupBy::None => EmitterGroupBy::None,
        },
        fields: match fields {
            FieldLevel::None => EmitterFieldLevel::None,
            FieldLevel::Keys => EmitterFieldLevel::Keys,
            FieldLevel::All => EmitterFieldLevel::All,
        },
        extension_filter: extension.map(str::to_string),
        kind_filter: (!kinds.is_empty()).then(|| kinds.to_vec()),
        root: root.map(str::to_string),
        depth,
    };

    let outcome = specforge_ops::model::model(&ProjectView::of(&project), &options);
    for warning in &outcome.warnings {
        eprintln!("{}", crate::export::render_plain(warning));
    }
    print!("{}", outcome.rendered);
    0
}
