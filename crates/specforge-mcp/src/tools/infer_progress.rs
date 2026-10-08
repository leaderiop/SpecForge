use specforge_ops::infer::Progress;

use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`), as `specforge infer-status --format
/// json` prints it.
pub fn call(call: &mut Call<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    let Ok(project) = call.project() else {
        let mut none = Progress::none().to_json();
        none["message"] = "No project root available".into();
        return ToolOutcome::ok(none);
    };
    match specforge_ops::infer::progress(&project.view()) {
        Ok(progress) => ToolOutcome::ok(progress.to_json()),
        Err(error) => crate::tool::McpError::from(error).into(),
    }
}
