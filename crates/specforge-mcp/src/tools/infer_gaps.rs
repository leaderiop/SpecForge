use serde_json::json;

use crate::target::Call;
use crate::tool::ToolOutcome;

/// `specforge.infer_gaps`: the shared gap report
/// (`specforge_ops::infer::gaps`), as `specforge infer-status --gaps-detail
/// --format json` prints it under `gap_analysis`.
pub fn call(call: &mut Call<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    let Ok(project) = call.project() else {
        return ToolOutcome::ok(json!({
            "total_pub_items": 0,
            "covered_items": 0,
            "gaps": [],
            "approximate": false,
            "message": "No project root available"
        }));
    };
    match specforge_ops::infer::gaps(&project.view(), project.runtime.as_ref()) {
        Ok(gaps) => ToolOutcome::ok(gaps.to_json()),
        Err(error) => crate::operations::op_error(error).into(),
    }
}
