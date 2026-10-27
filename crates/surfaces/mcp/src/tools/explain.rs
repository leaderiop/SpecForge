use crate::target::View;
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::Answered;
use specforge_ops::read::explain::ExplainField;
use specforge_ops::view::ProjectView;

/// `specforge.explain`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// A diagnostic code, any case
    code: String,
}

/// `specforge.explain`'s reply: the document `specforge explain --format json` prints, a
/// code book's entry (the catalog's, or a served project extension's own), or a retired code and
/// the entry that replaced it.
pub use specforge_ops::read::explain::Explanation as Reply;

/// `specforge.explain`: the explain operation (`specforge_ops::read::explain`) over the served
/// project's code book (the catalog alone when none is served), as data. A code the book neither has
/// nor retired is invalid input naming the `code` argument.
pub(crate) struct Tool;

impl ArgumentOf<ExplainField> for Tool {
    fn argument(field: ExplainField) -> Option<&'static str> {
        match field {
            ExplainField::Code => Some("code"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.explain";
    const DESCRIPTION: &'static str = "Explain a diagnostic code: its title, owner, level, what triggers it and how to fix it, and its docs link; a code the served project's extensions catalog, from their catalogs";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpExplainResult");
    type Args = Args;
    type Input = View;
    type Returns = Answered<Reply>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<Reply> {
        specforge_ops::read::explain::explain(&args.code, view.registries().codes())
            .map(Into::into)
            .map_err(|refusal| Box::new(Self::refused(refusal)))
    }
}
