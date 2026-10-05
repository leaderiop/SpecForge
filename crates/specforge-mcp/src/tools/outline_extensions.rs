use serde::Deserialize;
use specforge_emitter::outline::{
    DependencyDepth, OutlineDetail, OutlineFormat, OutlineIntermediate_from_manifests,
    OutlineOptions, render,
};

use crate::args::lenient;
use crate::state::McpState;
use crate::tool::ToolOutcome;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "lenient")]
    format: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    fields: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    deps: Option<String>,
}

pub fn call(state: &McpState, args: Args) -> ToolOutcome {
    let format = args.format.as_deref().unwrap_or("json");
    let fields = args.fields.as_deref().unwrap_or("keys");
    let deps = args.deps.as_deref().unwrap_or("direct");

    let outline_format = match format {
        "markdown" => OutlineFormat::Markdown,
        "mermaid" => OutlineFormat::Mermaid,
        "dot" => OutlineFormat::Dot,
        "json" => OutlineFormat::Json,
        _ => {
            return ToolOutcome::invalid_input(
                "format",
                format!(
                    "Unknown format: {}. Expected: markdown, mermaid, dot, json",
                    format
                ),
            );
        }
    };

    let detail = match fields {
        "none" => OutlineDetail::None,
        "keys" => OutlineDetail::Keys,
        "all" => OutlineDetail::All,
        _ => {
            return ToolOutcome::invalid_input(
                "fields",
                format!("Unknown fields: {}. Expected: none, keys, all", fields),
            );
        }
    };

    let dep_depth = match deps {
        "direct" => DependencyDepth::Direct,
        "effective" => DependencyDepth::Effective,
        "full" => DependencyDepth::Full,
        _ => {
            return ToolOutcome::invalid_input(
                "deps",
                format!("Unknown deps: {}. Expected: direct, effective, full", deps),
            );
        }
    };

    let options = OutlineOptions {
        format: outline_format,
        detail,
        deps: dep_depth,
    };

    let outline = OutlineIntermediate_from_manifests(&state.environment().manifests);
    let output = render(&outline, &options);

    ToolOutcome::text(output)
}
