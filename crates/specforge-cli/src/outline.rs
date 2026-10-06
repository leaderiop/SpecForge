use std::path::Path;

use specforge_emitter::outline::{
    DependencyDepth as EmitterDependencyDepth, OutlineDetail as EmitterOutlineDetail,
    OutlineFormat as EmitterOutlineFormat, OutlineOptions,
};
use specforge_ops::view::ProjectView;

use crate::pipeline;
use crate::{DepsLevel, FieldLevel, OutlineFormat};

/// `specforge outline`: the outline operation over the project compiled at
/// `path`.
pub fn run(path: &Path, format: OutlineFormat, fields: FieldLevel, deps: DepsLevel) -> i32 {
    let (project, _runtime) = pipeline::compile_project(path);
    let options = OutlineOptions {
        format: match format {
            OutlineFormat::Markdown => EmitterOutlineFormat::Markdown,
            OutlineFormat::Mermaid => EmitterOutlineFormat::Mermaid,
            OutlineFormat::Dot => EmitterOutlineFormat::Dot,
            OutlineFormat::Json => EmitterOutlineFormat::Json,
        },
        detail: match fields {
            FieldLevel::None => EmitterOutlineDetail::None,
            FieldLevel::Keys => EmitterOutlineDetail::Keys,
            FieldLevel::All => EmitterOutlineDetail::All,
        },
        deps: match deps {
            DepsLevel::Direct => EmitterDependencyDepth::Direct,
            DepsLevel::Effective => EmitterDependencyDepth::Effective,
            DepsLevel::Full => EmitterDependencyDepth::Full,
        },
    };
    print!(
        "{}",
        specforge_ops::model::outline(&ProjectView::of(&project), &options)
    );
    0
}
