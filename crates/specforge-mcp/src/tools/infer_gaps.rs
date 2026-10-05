use serde_json::json;

use crate::state::McpState;
use crate::tool::ToolOutcome;

/// `specforge.infer_gaps`: the shared gap report
/// (`specforge_ops::infer::gaps`), as `specforge infer-status --gaps-detail
/// --format json` prints it under `gap_analysis`.
pub fn call(state: &McpState, _args: crate::args::NoArgs) -> ToolOutcome {
    let Some(root) = state.project_root.clone() else {
        return ToolOutcome::ok(json!({
            "total_pub_items": 0,
            "covered_items": 0,
            "gaps": [],
            "approximate": false,
            "message": "No project root available"
        }));
    };
    let runtime = state.wasm_runtime(&root);
    match specforge_ops::infer::gaps(
        &root,
        state.registries().declarations(),
        state.graph(),
        runtime.as_ref(),
    ) {
        Ok(gaps) => ToolOutcome::ok(gaps.to_json()),
        Err(error) => crate::operations::op_error(error).into(),
    }
}
