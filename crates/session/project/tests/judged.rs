//! Judged rules (ADR 0290): what a project with none reads, and the pins of the ones it will.

use std::sync::Arc;

use specforge_project::{SharedRuntime, Watched};
use specforge_test::prelude::*;

use crate::check_passes::{SPEC, passes_extension, project};

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a project with no judged rule reads no report and stamps no file"
)]
fn a_project_without_judged_rules_reads_no_report_and_stamps_no_file() {
    let dir = project(SPEC).file("specforge-report.json", "{}");
    let ext = Arc::new(passes_extension());
    let session = dir.session_in(Arc::clone(&ext) as SharedRuntime);
    let report = dir.root().join("specforge-report.json");
    let watched = session.inputs().watched();
    assert!(!watched.contains(&Watched::File(report)), "{watched:?}");
    let codes: Vec<_> = session
        .project()
        .diagnostics()
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    assert!(
        !codes.iter().any(|c| c == "W168" || c == "I205"),
        "{codes:?}"
    );
}
