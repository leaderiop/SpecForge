//! The plan rules, end to end: each fires on its fixture and is silent on its counterpart. Every test asserts the
//! full `codes()` list, kernel codes included, and the entity the diagnostic names: a `findings()`-only assertion
//! would not see a stray kernel error.

use crate::harness::{Project, project};

const CLEAN: Vec<String> = Vec::new();

/// The message of the one `gsd/` finding `code`, the test having asserted there is exactly that one.
fn only(p: &Project, code: &str) -> String {
    let found = p.findings();
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, code, "{found:#?}");
    found[0].1.clone()
}

fn strict_fails(p: &Project, code: &str) {
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains(code),
        "{code} in check: {}",
        check.all()
    );
}

const FORGOTTEN: &str = "plan 03-01 \"Forgot its phase\" {\n  type execute\n  files_modified [\"src/x.js\"]\n  objective \"x\"\n}\n";

#[test]
fn the_plan_project_reports_no_plan_rule() {
    assert_eq!(project("plan").codes(), CLEAN);
}

#[test]
fn a_plan_without_its_phase_is_w323() {
    let p = project("plan");
    p.write("spec/forgotten.spec", FORGOTTEN);
    assert_eq!(p.codes(), ["gsd/W323"]);
    let message = only(&p, "gsd/W323");
    assert!(
        message.contains("plan '03-01' has no phase and is not named by a quick id"),
        "{message}"
    );
}

#[test]
fn a_plan_named_after_another_phase_is_w323() {
    let p = project("plan");
    // Every mention of the plan follows its new name; only the name stops matching its phase.
    let spec = p.read("spec/plans.spec");
    assert!(spec.contains("02-01"));
    p.write("spec/plans.spec", &spec.replace("02-01", "01-09"));
    assert_eq!(p.codes(), ["gsd/W323"]);
    let message = only(&p, "gsd/W323");
    assert!(
        message.contains("plan 'v1.0/02/01-09' is in phase 'v1.0/02' but is not named 02-NN"),
        "{message}"
    );
}

#[test]
fn w323_is_an_error_under_gsd_strict() {
    let p = project("plan");
    p.write("spec/forgotten.spec", FORGOTTEN);
    strict_fails(&p, "W323");
}
