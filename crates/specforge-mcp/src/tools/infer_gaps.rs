use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

/// `specforge.infer_gaps`: the shared gap report
/// (`specforge_ops::infer::gaps`), as `specforge infer-status --gaps-detail
/// --format json` prints it under `gap_analysis`. With no project served it
/// is the no-project refusal (the call target's), as the operation refuses
/// a view without a root.
pub fn call(project: &ProjectRef<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    match specforge_ops::infer::gaps(&project.view(), project.runtime.as_ref()) {
        Ok(gaps) => ToolOutcome::ok(gaps.to_json()),
        Err(error) => crate::tool::McpError::from(error).into(),
    }
}
