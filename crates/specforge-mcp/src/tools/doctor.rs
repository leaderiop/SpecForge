//! `specforge.doctor`: the health report (`specforge_ops::doctor`).

use crate::args::NoArgs;
use crate::target::ProjectRef;
use crate::tool::ToolOutcome;

pub(crate) fn call(project: &ProjectRef<'_>, _args: NoArgs) -> ToolOutcome {
    // The target brought the project up to date with disk unless the
    // caller opted into the last compile (`use_cached`, ADR 0004 D3-d).
    // The same report `specforge doctor` prints (`DoctorReport::to_json`), as
    // the spec's McpDoctorReport. Credential health is the user's, not the
    // project's: only the CLI reports it.
    let report = specforge_ops::doctor::diagnose(&project.view());
    ToolOutcome::ok(report.to_json())
}
