//! `specforge.diff`: the nodes and edges that changed between two snapshots of the project, the answer of
//! `specforge diff` (ADR 0376 D5).

use crate::args::Arguments;
use crate::refusal::ArgumentOf;
use crate::reply::Answered;
use crate::target::{Project, ProjectRef};
use crate::tool::{CoreTool, Effect, ReplySpec, ToolGroup};
use specforge_ops::read::diff::{DiffAnswer, DiffField, DiffRequest, diff};

/// `specforge.diff`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// What to compare: `<rev>` is that revision against the working tree, `<a>..<b>` two revisions; none is HEAD
    /// against the working tree
    range: Option<String>,
    /// Keep only the changes of these kinds: their nodes and the edges with an end of one
    kinds: Vec<String>,
}

pub(crate) struct Tool;

impl ArgumentOf<DiffField> for Tool {
    fn argument(field: DiffField) -> Option<&'static str> {
        match field {
            DiffField::Range => Some("range"),
        }
    }
}

impl CoreTool for Tool {
    const NAME: &'static str = "specforge.diff";
    const DESCRIPTION: &'static str = "Name the nodes and edges that changed between two snapshots of the project, with the digests of each changed node (range: <rev> against the working tree, <a>..<b> two revisions, none HEAD against the working tree; kinds: keep only those kinds' changes)";
    const EFFECT: Effect = Effect::Reads {
        group: ToolGroup::Core,
    };
    const SPEC: ReplySpec = ReplySpec::Type("McpDiffResult");
    type Args = Args;
    type Input = Project;
    type Returns = Answered<DiffAnswer>;

    fn call(project: ProjectRef<'_>, args: Args) -> Answered<DiffAnswer> {
        let request = DiffRequest {
            range: args.range,
            kinds: args.kinds,
        };
        Ok(diff(&project.view(), &request)
            .map_err(Self::refused)?
            .into())
    }
}
