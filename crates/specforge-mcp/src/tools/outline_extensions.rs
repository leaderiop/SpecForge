use specforge_ops::model::{self, DependencyDepth, OutlineDetail, OutlineFormat, OutlineOptions};

use crate::args::Arguments;
use crate::tool::ToolOutcome;
use specforge_ops::view::ProjectView;

/// `specforge.outline_extensions`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Output format; json is meant for programs
    #[arg(choice = specforge_ops::model::OUTLINE_FORMAT)]
    format: OutlineFormat,
    /// Detail level
    #[arg(choice = specforge_ops::model::OUTLINE_FIELDS)]
    fields: OutlineDetail,
    /// Dependency visibility
    #[arg(choice = specforge_ops::model::DEPS)]
    deps: DependencyDepth,
}

/// `specforge.outline_extensions`: the outline operation over the served
/// project.
pub fn call(view: ProjectView<'_>, args: Args) -> ToolOutcome {
    ToolOutcome::text(model::outline(&view, &options(&args)))
}

/// The outline options the arguments name; an absent one is its table's
/// default, as `specforge outline` reads it (ADR 0027).
fn options(args: &Args) -> OutlineOptions {
    OutlineOptions {
        format: args.format,
        detail: args.fields,
        deps: args.deps,
    }
}
