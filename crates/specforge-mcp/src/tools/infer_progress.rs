use crate::reply::Answered;
use crate::target::ProjectRef;
use crate::tool::McpError;

/// `specforge.infer_progress`'s reply: the document `specforge infer-status
/// --format json` prints (`McpInferProgressResult`).
pub use specforge_ops::infer::ProgressDocument as Reply;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`). With no project served it is the
/// no-project refusal (the call target's), as the operation refuses a view
/// without a root.
pub fn call(project: &ProjectRef<'_>, _args: crate::args::NoArgs) -> Answered<Reply> {
    let progress = specforge_ops::infer::progress(&project.view()).map_err(McpError::from)?;
    Ok(progress.document().into())
}
