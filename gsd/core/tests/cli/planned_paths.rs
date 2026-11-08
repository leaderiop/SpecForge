//! A plan's paths, end to end: what the plan will create is checked once the plan is complete.

use crate::harness::project;

#[test]
fn a_complete_plans_missing_planned_file_reports_nothing() {
    let p = project("execute");
    assert!(!p.exists("src/tasks.js"));
    assert_eq!(p.fields_of("v1.0/01/01-01")["status"], "complete");
    assert_eq!(p.codes(), Vec::<String>::new());
}
