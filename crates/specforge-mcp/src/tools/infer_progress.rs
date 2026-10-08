use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`), as `specforge infer-status --format
/// json` prints it. With no project served it is the no-project refusal
/// (the call target's), as the operation refuses a view without a root.
pub fn call(project: &ProjectRef<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    match specforge_ops::infer::progress(&project.view()) {
        Ok(progress) => ToolOutcome::ok(progress.to_json()),
        Err(error) => crate::tool::McpError::from(error).into(),
    }
}
