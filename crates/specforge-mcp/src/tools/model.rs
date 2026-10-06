use serde::Deserialize;
use specforge_ops::model::{self, ModelOptions};

use crate::args::{choice, lenient, some_strings};
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

/// `specforge.model`: the model operation over the served project; its
/// W146 warnings are the result's diagnostics.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let options = match options(args) {
        Ok(options) => options,
        Err(refused) => return refused,
    };
    let outcome = model::model(&call.view(), &options);
    ToolOutcome::text(outcome.rendered).with_diagnostics(outcome.warnings)
}

/// The model options the arguments name; an absent enumerated one is its
/// table's default, as `specforge model` reads it (ADR 0027).
fn options(args: Args) -> Result<ModelOptions, ToolOutcome> {
    Ok(ModelOptions {
        format: choice(&model::MODEL_FORMAT, "format", args.format.as_deref())?,
        group_by: choice(&model::GROUP_BY, "group_by", args.group_by.as_deref())?,
        fields: choice(&model::MODEL_FIELDS, "fields", args.fields.as_deref())?,
        extension_filter: args.extension,
        kind_filter: args.kinds,
        root: args.root,
        depth: args.depth.map(|d| d as usize),
    })
}
