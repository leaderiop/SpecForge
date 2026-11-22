//! `specforge.context`: the pack `specforge context` prints (ADR 0356), through the same operation.

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::{Answered, Text};
use crate::target::View;
use crate::tool::{CoreTool, Effect, ToolGroup};
use specforge_ops::read::context::{ContextField, ContextRequest, context};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;

/// `specforge.context`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// The context route, by its qualified name (`<extension>/<route>`)
    route: String,
    /// The entity to read it for: one of the route's subject kind
    subject: String,
    /// A cap on the whole pack, refused below the subject and the core sections; without one they are served whole and
    /// enrich sections fill the route's allowance
    max_tokens: Option<usize>,
    /// Format of the pack's graph
    #[arg(choice = specforge_ops::read::context::PACK_FORMAT)]
    format: Format,
}

pub(crate) struct Tool;

impl ArgumentOf<ContextField> for Tool {
    fn argument(field: ContextField) -> Option<&'static str> {
        match field {
            ContextField::Route => Some("route"),
            ContextField::Subject => Some("subject"),
            ContextField::MaxTokens => Some("max_tokens"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.context";
    const DESCRIPTION: &'static str = "Read the context a route declares for one entity: its core sections whole, then its enrich sections in declared order within the route's allowance, with a digest";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    type Args = Args;
    type Input = View;
    type Returns = Answered<Text>;

    fn call(view: ProjectView<'_>, args: Args) -> Answered<Text> {
        let request = ContextRequest {
            route: &args.route,
            subject: &args.subject,
            max_tokens: args.max_tokens,
            format: args.format,
        };
        let pack = context(&view, &request).map_err(Self::refused)?;
        Ok(Text(pack.text).into())
    }
}
