use serde::Deserialize;
use specforge_emitter::model::{FieldLevel, GroupBy, ModelFormat, ModelOptions};
use specforge_ops::model::Named;

use crate::args::{lenient, some_strings};
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

/// An argument's name parsed by the operation's rule, or the tool's
/// `invalid_input` refusal on that argument.
pub(super) fn parse<T>(argument: &str, name: &str) -> Result<T, ToolOutcome>
where
    Named<T>: std::str::FromStr<Err = specforge_ops::OpError>,
{
    name.parse::<Named<T>>()
        .map(|Named(value)| value)
        .map_err(|error| {
            crate::operations::op_error(error)
                .with_argument(argument)
                .into()
        })
}

/// `specforge.model`: the model operation over the served project; its
/// W146 warnings are the result's diagnostics.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let options = match options(args) {
        Ok(options) => options,
        Err(refused) => return refused,
    };
    let outcome = specforge_ops::model::model(&call.view(), &options);
    ToolOutcome::text(outcome.rendered).with_diagnostics(outcome.warnings)
}

/// The model options the arguments name (defaults: markdown, grouped by
/// extension, key fields).
fn options(args: Args) -> Result<ModelOptions, ToolOutcome> {
    Ok(ModelOptions {
        format: parse::<ModelFormat>("format", args.format.as_deref().unwrap_or("markdown"))?,
        group_by: parse::<GroupBy>("group_by", args.group_by.as_deref().unwrap_or("extension"))?,
        fields: parse::<FieldLevel>("fields", args.fields.as_deref().unwrap_or("keys"))?,
        extension_filter: args.extension,
        kind_filter: args.kinds,
        root: args.root,
        depth: args.depth.map(|d| d as usize),
    })
}
