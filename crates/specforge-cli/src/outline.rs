use std::path::Path;

use specforge_emitter::outline::{
    DependencyDepth as EmitterDependencyDepth, OutlineDetail as EmitterOutlineDetail,
    OutlineFormat as EmitterOutlineFormat, OutlineIntermediate_from_declarations, OutlineOptions,
    render,
};

use crate::pipeline;
use crate::{DepsLevel, FieldLevel, OutlineFormat};

pub fn run(path: &Path, format: OutlineFormat, fields: FieldLevel, deps: DepsLevel) -> i32 {
    let ctx = pipeline::compile(path);

    let outline_format = match format {
        OutlineFormat::Markdown => EmitterOutlineFormat::Markdown,
        OutlineFormat::Mermaid => EmitterOutlineFormat::Mermaid,
        OutlineFormat::Dot => EmitterOutlineFormat::Dot,
        OutlineFormat::Json => EmitterOutlineFormat::Json,
    };

    let detail = match fields {
        FieldLevel::None => EmitterOutlineDetail::None,
        FieldLevel::Keys => EmitterOutlineDetail::Keys,
        FieldLevel::All => EmitterOutlineDetail::All,
    };

    let dep_depth = match deps {
        DepsLevel::Direct => EmitterDependencyDepth::Direct,
        DepsLevel::Effective => EmitterDependencyDepth::Effective,
        DepsLevel::Full => EmitterDependencyDepth::Full,
    };

    let options = OutlineOptions {
        format: outline_format,
        detail,
        deps: dep_depth,
    };

    let outline = OutlineIntermediate_from_declarations(&ctx.declarations);
    let output = render(&outline, &options);
    print!("{}", output);

    0
}
