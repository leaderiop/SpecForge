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

/// A plan with no `phase`, named like a phase plan, with the must-haves W309 wants.
const FORGOTTEN: &str = "plan 03-01 \"Forgot its phase\" {\n  type execute\n  files_modified [\"src/x.js\"]\n  objective \"x\"\n}\n\ntruth TR1 \"x\" {\n  plan 03-01\n  text \"x\"\n}\n\nartifact AR1 \"x\" {\n  plan 03-01\n  path \"src/x.js\"\n  provides \"x\"\n}\n";

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

/// A fresh copy of the plan project with `more` appended as `spec/more.spec`.
fn with(more: &str) -> Project {
    let p = project("plan");
    p.write("spec/more.spec", more);
    p
}

fn manual(name: &str, plan: &str) -> String {
    format!(
        "task {name} \"Manual {name}\" {{\n  plan   {plan}\n  type   manual\n  action \"Do it by hand\"\n  done   \"Done by hand\"\n}}\n\n"
    )
}

const T1_FILES: &str =
    "  files  [\"src/tasks.js\"]\n  action \"Write src/tasks.js with add() and list()\"\n";
const T2_VERIFY: &str = "  verify [\n    { automated \"node --test test/tasks.test.js\" fails_when \"a test fails or none runs\" }\n  ]\n";

#[test]
fn each_plan_shape_code_fires_on_its_fixture() {
    // E303: an auto task without files.
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        T1_FILES,
        "  action \"Write src/tasks.js with add() and list()\"\n",
    );
    assert_eq!(p.codes(), ["gsd/E303"]);
    assert!(only(&p, "gsd/E303").contains("task 'v1.0/01/01-01/T1' of type auto has no files"));

    // E303: a tdd task with no verify of any form.
    let p = project("plan");
    p.edit("spec/plans.spec", T2_VERIFY, "");
    assert_eq!(p.codes(), ["gsd/E303"]);
    assert!(only(&p, "gsd/E303").contains("task 'v1.0/01/01-01/T2' of type tdd has no a verify"));

    // W304: four children, and E322: five. Never both.
    let four = format!(
        "{}{}",
        manual("T3", "v1.0/01/01-01"),
        manual("T4", "v1.0/01/01-01")
    );
    let p = with(&four);
    assert_eq!(p.codes(), ["gsd/W304"]);
    assert!(only(&p, "gsd/W304").contains("plan 'v1.0/01/01-01' has 4 tasks"));
    let p = with(&format!("{four}{}", manual("T5", "v1.0/01/01-01")));
    assert_eq!(p.codes(), ["gsd/E322"]);
    assert!(only(&p, "gsd/E322").contains("plan 'v1.0/01/01-01' has 5 tasks"));

    // E307: a human_verify without how_to_verify, a human_action without verification.
    let p = with(
        "checkpoint C2 \"Look\" {\n  plan          v1.0/02/02-01\n  kind          human_verify\n  what_built    \"The store\"\n  resume_signal \"approved\"\n}\n",
    );
    assert_eq!(p.codes(), ["gsd/E307"]);
    assert!(
        only(&p, "gsd/E307")
            .contains("checkpoint 'v1.0/02/02-01/C2' of kind human_verify has no how_to_verify")
    );
    let p = with(
        "checkpoint C2 \"Do\" {\n  plan          v1.0/02/02-01\n  kind          human_action\n  instructions  \"Create the directory\"\n  action        \"mkdir data\"\n  resume_signal \"done\"\n}\n",
    );
    assert_eq!(p.codes(), ["gsd/E307"]);
    assert!(only(&p, "gsd/E307").contains("of kind human_action has no verification"));

    // E307: an auto_select outside the options names the available ones.
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  kind          decision\n",
        "  kind          decision\n  auto_select   \"c\"\n",
    );
    assert_eq!(p.codes(), ["gsd/E307"]);
    assert!(
        only(&p, "gsd/E307")
            .contains("auto_select=\"c\" does not match any option (available: flag, timestamp)")
    );
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  kind          decision\n",
        "  kind          decision\n  auto_select   \"flag\"\n",
    );
    assert_eq!(p.codes(), CLEAN, "a matching auto_select is clean");

    // W308: a one_way task with no decision before it.
    let door = "  reversibility { rating one_way why \"the file format\" }\n";
    let p = project("plan");
    p.edit("spec/plans.spec", T1_FILES, &format!("{T1_FILES}{door}"));
    assert_eq!(p.codes(), ["gsd/W308"]);
    assert!(
        only(&p, "gsd/W308")
            .contains("task 'v1.0/01/01-01/T1' rated one_way has no preceding checkpoint:decision")
    );

    // ... the same with settled_by.
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        T1_FILES,
        &format!(
            "{T1_FILES}  reversibility {{ rating one_way why \"the format\" settled_by /D-01 }}\n"
        ),
    );
    assert_eq!(p.codes(), CLEAN);

    // ... and after a decision checkpoint (`C1` of plan 01-02 is after T1).
    let p = with(
        "task T2 \"Irreversible\" {\n  plan          v1.0/01/01-02\n  type          manual\n  reversibility { rating one_way why \"deletes the old file\" }\n  action        \"Delete it\"\n  done          \"Deleted\"\n}\n",
    );
    assert_eq!(p.codes(), CLEAN);

    // W309: a plan with no artifact.
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "artifact AR1 \"Store module\" {\n  plan     v1.0/02/02-01\n  path     \"src/store.js\"\n  provides \"save() and load()\"\n}\n",
        "",
    );
    assert_eq!(p.codes(), ["gsd/W309"]);
    assert!(only(&p, "gsd/W309").contains("plan 'v1.0/02/02-01' has no artifact"));

    // E319: a prohibition checked by a test names it.
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "verification judgment",
        "verification test",
    );
    assert_eq!(p.codes(), ["gsd/E319"]);
    assert!(
        only(&p, "gsd/E319")
            .contains("prohibition 'v1.0/02/02-01/PR1' is verified by a test but names none")
    );
    p.edit(
        "spec/plans.spec",
        "verification test",
        "verification test\n  test         \"node --test test/store.test.js\"",
    );
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn a_manual_task_without_files_and_three_children_are_clean() {
    let p = with(&manual("T3", "v1.0/01/01-01"));
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn w308_is_an_error_under_gsd_strict() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        T1_FILES,
        &format!("{T1_FILES}  reversibility {{ rating one_way why \"the file format\" }}\n"),
    );
    strict_fails(&p, "W308");
}
