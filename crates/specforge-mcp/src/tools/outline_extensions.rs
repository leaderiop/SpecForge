use serde::Deserialize;
use specforge_ops::model::{self, OutlineOptions};

use crate::args::{choice, lenient};
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
        Ok(options) => ToolOutcome::text(model::outline(&call.view(), &options)),
        Err(refused) => refused,
    }
}

/// The outline options the arguments name; an absent one is its table's
/// default, as `specforge outline` reads it (ADR 0027).
fn options(args: &Args) -> Result<OutlineOptions, ToolOutcome> {
    Ok(OutlineOptions {
        format: choice(&model::OUTLINE_FORMAT, "format", args.format.as_deref())?,
        detail: choice(&model::OUTLINE_FIELDS, "fields", args.fields.as_deref())?,
        deps: choice(&model::DEPS, "deps", args.deps.as_deref())?,
    })
}
