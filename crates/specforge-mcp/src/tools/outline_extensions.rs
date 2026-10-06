use serde::Deserialize;
use specforge_emitter::outline::{DependencyDepth, OutlineDetail, OutlineFormat, OutlineOptions};

use super::model::parse;
use crate::args::lenient;
use crate::target::Call;
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

/// `specforge.outline_extensions`: the outline operation over the served
/// project.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    match options(&args) {
        Ok(options) => ToolOutcome::text(specforge_ops::model::outline(&call.view(), &options)),
        Err(refused) => refused,
    }
}

/// The outline options the arguments name (defaults: json, key fields,
/// direct dependencies).
fn options(args: &Args) -> Result<OutlineOptions, ToolOutcome> {
    Ok(OutlineOptions {
        format: parse::<OutlineFormat>("format", args.format.as_deref().unwrap_or("json"))?,
        detail: parse::<OutlineDetail>("fields", args.fields.as_deref().unwrap_or("keys"))?,
        deps: parse::<DependencyDepth>("deps", args.deps.as_deref().unwrap_or("direct"))?,
    })
}
