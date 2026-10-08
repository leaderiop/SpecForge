use crate::target::Call;
use crate::tool::Handled;
use crate::tool::ToolOutcome;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`), as `specforge infer-status --format
/// json` prints it. With no project served it is the no-project refusal, as
/// the operation refuses a view without a root.
pub fn call(call: &mut Call<'_>, _args: crate::args::NoArgs) -> Handled {
    let project = call.project()?;
    Ok(match specforge_ops::infer::progress(&project.view()) {
        Ok(progress) => ToolOutcome::ok(progress.to_json()),
        Err(error) => crate::tool::McpError::from(error).into(),
    })
}
