use serde_json::json;

use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`), as `specforge infer-status --format
/// json` prints it.
pub fn call(call: &mut Call<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    let Ok(project) = call.project() else {
        return ToolOutcome::ok(json!({
            "summary": { "files_total": 0, "files_analyzed": 0, "entities_produced": 0 },
            "unanalyzed": [],
            "stale": [],
            "deleted": [],
            "message": "No project root available"
        }));
    };
    match specforge_ops::infer::progress(&project.view()) {
        Ok(progress) => ToolOutcome::ok(progress.to_json()),
        Err(error) => crate::tool::McpError::from(error).into(),
    }
}
