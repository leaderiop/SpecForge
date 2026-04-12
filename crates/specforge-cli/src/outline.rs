use std::path::Path;

use specforge_emitter::outline::{
    render, DependencyDepth, OutlineDetail, OutlineFormat, OutlineIntermediate_from_manifests,
    OutlineOptions,
};

use crate::pipeline;

pub fn run(path: &Path, format: &str, fields: &str, deps: &str) -> i32 {
    let ctx = pipeline::compile(path);

    let outline_format = match format {
        "mermaid" => OutlineFormat::Mermaid,
        "dot" => OutlineFormat::Dot,
        "json" => OutlineFormat::Json,
        _ => OutlineFormat::Markdown,
    };

    let detail = match fields {
        "none" => OutlineDetail::None,
        "all" => OutlineDetail::All,
        _ => OutlineDetail::Keys,
    };

    let dep_depth = match deps {
        "effective" => DependencyDepth::Effective,
        "full" => DependencyDepth::Full,
        _ => DependencyDepth::Direct,
    };

    let options = OutlineOptions {
        format: outline_format,
        detail,
        deps: dep_depth,
    };

    let outline = OutlineIntermediate_from_manifests(&ctx.manifests);
    let output = render(&outline, &options);
    print!("{}", output);

    0
}
