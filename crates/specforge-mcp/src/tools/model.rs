use specforge_ops::OpError;
use specforge_ops::model::{self, FieldLevel, GroupBy, ModelFormat, ModelOptions, ModelRoot};

use crate::args::Arguments;
use crate::tool::{McpError, ToolOutcome};
use specforge_ops::view::ProjectView;

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

/// `specforge.model`: the model operation over the served project. A kind
/// of `kinds` the project does not know rides in `_meta.diagnostics`
/// (I020). An unknown `root` or `extension` is refused on that argument,
/// and so is a `depth` without a `root`.
pub fn call(view: ProjectView<'_>, args: Args) -> ToolOutcome {
    let options = match options(args) {
        Ok(options) => options,
        Err(refused) => return refused,
    };
    match model::model(&view, &options) {
        Ok(outcome) => ToolOutcome::text(outcome.document).with_diagnostics(outcome.notices),
        Err(error) => {
            let argument = argument_of(&error);
            let error = McpError::from(error);
            match argument {
                Some(argument) => error.with_argument(argument),
                None => error,
            }
            .into()
        }
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

/// The argument a model refusal is about.
fn argument_of(error: &OpError) -> Option<&'static str> {
    match error.code.as_ref() {
        specforge_ops::view::UNKNOWN_KIND => Some("root"),
        specforge_ops::extension::NOT_FOUND => Some("extension"),
        _ => None,
    }
}
