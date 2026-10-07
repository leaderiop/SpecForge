use specforge_ops::model::{self, FieldLevel, GroupBy, ModelFormat, ModelOptions};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.model`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Output format
    #[arg(choice = specforge_ops::model::MODEL_FORMAT)]
    format: ModelFormat,
    /// How to group entities
    #[arg(choice = specforge_ops::model::GROUP_BY)]
    group_by: GroupBy,
    /// Field detail level
    #[arg(choice = specforge_ops::model::MODEL_FIELDS)]
    fields: FieldLevel,
    /// Filter to a single extension
    extension: Option<String>,
    /// Filter to specific entity kinds
    kinds: Option<Vec<String>>,
    /// Root entity kind for depth-scoped output
    root: Option<String>,
    /// Maximum depth from root kind (requires root)
    depth: Option<usize>,
}

/// `specforge.model`: the model operation over the served project; its
/// W146 warnings are the result's diagnostics.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    let outcome = model::model(&call.view(), &options(args));
    ToolOutcome::text(outcome.rendered).with_diagnostics(outcome.warnings)
}

/// The model options the arguments name; an absent enumerated one is its
/// table's default, as `specforge model` reads it (ADR 0027).
fn options(args: Args) -> ModelOptions {
    ModelOptions {
        format: args.format,
        group_by: args.group_by,
        fields: args.fields,
        extension_filter: args.extension,
        kind_filter: args.kinds,
        root: args.root,
        depth: args.depth,
    }
}
