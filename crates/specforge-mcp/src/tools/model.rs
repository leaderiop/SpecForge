use specforge_ops::model::{self, FieldLevel, GroupBy, ModelFormat, ModelOptions, ModelRoot};

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
    /// Filter to specific entity kinds (empty: every kind)
    kinds: Vec<String>,
    /// Root entity kind for depth-scoped output
    root: Option<String>,
    /// Maximum depth from root kind (requires root)
    depth: Option<usize>,
}

/// `specforge.model`: the model operation over the served project. A
/// `depth` without a `root` is refused on `root`, as `--depth` needs
/// `--root`.
pub fn call(call: &mut Call<'_>, args: Args) -> ToolOutcome {
    match options(args) {
        Ok(options) => ToolOutcome::text(model::model(&call.view(), &options)),
        Err(refused) => refused,
    }
}

/// The model options the arguments name; an absent enumerated one is its
/// table's default, as `specforge model` reads it (ADR 0027). A `depth`
/// needs a `root`.
fn options(args: Args) -> Result<ModelOptions, ToolOutcome> {
    let root = match (args.root, args.depth) {
        (Some(kind), depth) => Some(ModelRoot { kind, depth }),
        (None, None) => None,
        (None, Some(_)) => {
            return Err(ToolOutcome::invalid_input(
                "root",
                "'depth' needs 'root': the kind the depth counts from",
            ));
        }
    };
    Ok(ModelOptions {
        format: args.format,
        group_by: args.group_by,
        fields: args.fields,
        extension: args.extension,
        kinds: args.kinds,
        root,
    })
}
