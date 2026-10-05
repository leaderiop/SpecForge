use serde_json::json;

use crate::state::McpState;
use crate::tool::ToolOutcome;

/// `specforge.infer_progress`: the shared progress document
/// (`specforge_ops::infer::progress`), as `specforge infer-status --format
/// json` prints it.
pub fn call(state: &McpState, _args: crate::args::NoArgs) -> ToolOutcome {
    let Some(root) = &state.project_root else {
        return ToolOutcome::ok(json!({
            "summary": { "files_total": 0, "files_analyzed": 0, "entities_produced": 0 },
            "unanalyzed": [],
            "stale": [],
            "deleted": [],
            "message": "No project root available"
        }));
    };
    match specforge_ops::infer::progress(root, &state.environment().manifests) {
        Ok(progress) => ToolOutcome::ok(progress.to_json()),
        Err(error) => crate::operations::op_error(error).into(),
    }
}
