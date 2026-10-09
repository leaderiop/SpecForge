//! `specforge.doctor`: the health report (`specforge_ops::doctor`).

use crate::args::NoArgs;
use crate::reply::Answered;
use crate::target::ProjectRef;

/// `specforge.doctor`'s reply: the document `specforge doctor --format json`
/// prints, without the user's credential health (`McpDoctorReport`).
pub use specforge_ops::doctor::DoctorDocument as Reply;

pub(crate) fn call(project: &ProjectRef<'_>, _args: NoArgs) -> Answered<Reply> {
    // The target brought the project up to date with disk unless the
    // caller opted into the last compile (`use_cached`, ADR 0004 D3-d).
    // The same report `specforge doctor` prints (`DoctorReport::document`), as
    // the spec's McpDoctorReport. Credential health is the user's, not the
    // project's: only the CLI reports it.
    let report = specforge_ops::doctor::diagnose(&project.view());
    Ok(report.document().into())
}
