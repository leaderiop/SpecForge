use crate::reply::Answered;
use crate::target::ProjectRef;
use crate::tool::McpError;

/// `specforge.infer_gaps`'s reply: the document `specforge infer-status
/// --gaps-detail --format json` prints under `gap_analysis`
/// (`McpInferGapsResult`).
pub use specforge_ops::infer::GapsDocument as Reply;

/// `specforge.infer_gaps`: the shared gap report
/// (`specforge_ops::infer::gaps`). With no project served it is the
/// no-project refusal (the call target's), as the operation refuses a view
/// without a root.
pub fn call(project: &ProjectRef<'_>, _args: crate::args::NoArgs) -> Answered<Reply> {
    let gaps = specforge_ops::infer::gaps(&project.view()).map_err(McpError::from)?;
    Ok(gaps.document().into())
}
