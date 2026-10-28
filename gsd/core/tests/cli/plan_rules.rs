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

/// The message of the `gsd/` finding `code`, among others.
fn message(p: &Project, code: &str) -> String {
    p.findings()
        .into_iter()
        .find(|(c, _)| c == code)
        .unwrap_or_else(|| panic!("no {code}"))
        .1
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

#[test]
fn plan_without_requirements_is_e302() {
    let p = project("plan");
    p.edit("spec/plans.spec", "  requirements   [TASK-01]\n", "");
    // The plan names no requirement: phase 01's TASK-01 is in no plan (E301), nor in any plan's text (W317).
    assert_eq!(p.codes(), ["gsd/E302", "gsd/E301", "gsd/W317"]);
    assert!(message(&p, "gsd/E302").contains("plan 'v1.0/01/01-01' names no requirement"));
}

#[test]
fn plan_with_empty_requirements_is_e302() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  requirements   [TASK-01]\n",
        "  requirements   []\n",
    );
    assert_eq!(p.codes(), ["gsd/E302", "gsd/E301", "gsd/W317"]);
}

#[test]
fn a_quick_plan_without_requirements_is_clean() {
    // The fixture's quick plan `261028-a1b` names none.
    let p = project("plan");
    assert!(p.read("spec/plans.spec").contains("plan 261028-a1b"));
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn plan_cycle_is_e102() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
        "  depends_on     [01-02]\n  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
    );
    // 01-01 and 01-02 depend on each other: the kernel's own W061 for the reference cycle, and one E102 per plan.
    assert_eq!(p.codes(), ["W061", "gsd/E102", "gsd/E102"]);
    let messages: Vec<String> = p.findings().into_iter().map(|(_, m)| m).collect();
    assert_eq!(
        messages,
        [
            "plan 'v1.0/01/01-01' is on a dependency cycle",
            "plan 'v1.0/01/01-02' is on a dependency cycle"
        ]
    );
}

#[test]
fn a_plan_depending_on_another_phase_is_e102() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/store.js\"]\n",
        "  depends_on     [v1.0/01/01-02]\n  files_modified [\"src/store.js\"]\n",
    );
    assert_eq!(p.codes(), ["gsd/E102"]);
    assert!(only(&p, "gsd/E102").contains(
        "plan 'v1.0/02/02-01' depends on 'v1.0/01/01-02', which is not a plan of its phase: GSD drops the edge"
    ));
}

/// A third plan of phase 01 with the must-haves W309 wants, modifying `modified` and deleting `deleted`.
fn third_plan(depends: &str, modified: &str, deleted: &str) -> String {
    format!(
        "plan 01-03 \"Third\" {{\n  phase          v1.0/01\n  type           execute\n{depends}  files_modified [{modified}]\n  files_deleted  [{deleted}]\n  requirements   [TASK-01]\n  objective      \"A third plan\"\n}}\n\ntruth TR1 \"Third\" {{\n  plan v1.0/01/01-03\n  text \"Third\"\n}}\n\nartifact AR1 \"Third\" {{\n  plan     v1.0/01/01-03\n  path     \"src/third.js\"\n  provides \"third\"\n}}\n"
    )
}

#[test]
fn same_wave_overlap_is_w305() {
    // 01-01 is wave 1, and so is a plan with no `depends_on` that also modifies `src/tasks.js`.
    let p = with(&third_plan("", "\"src/tasks.js\"", ""));
    assert_eq!(p.codes(), ["gsd/W305"]);
    assert!(only(&p, "gsd/W305").contains(
        "plans 'v1.0/01/01-01' and 'v1.0/01/01-03' of wave 1 both modify 'src/tasks.js': under gsd-compat the wave runs sequentially"
    ));
}

#[test]
fn a_deleted_file_counts_as_overlap() {
    let p = with(&third_plan("", "\"src/third.js\"", "\"src/tasks.js\""));
    assert_eq!(p.codes(), ["gsd/W305"]);
}

#[test]
fn plans_of_different_waves_may_share_a_file() {
    let p = with(&third_plan(
        "  depends_on     [01-01]\n",
        "\"src/tasks.js\"",
        "",
    ));
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn a_path_holding_a_comma_is_one_item() {
    // One item `src/tasks.js,test/tasks.test.js`: equal to neither of 01-01's two paths.
    let p = with(&third_plan("", "\"src/tasks.js,test/tasks.test.js\"", ""));
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn task_file_outside_plan_is_w306() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files  [\"src/tasks.js\"]\n  action \"Write src/tasks.js",
        "  files  [\"src/tasks.js\", \"src/extra.js\"]\n  action \"Write src/tasks.js",
    );
    assert_eq!(p.codes(), ["gsd/W306"]);
    assert!(only(&p, "gsd/W306").contains(
        "task 'v1.0/01/01-01/T1' names 'src/extra.js', which is not among the files of plan 'v1.0/01/01-01'"
    ));
}

#[test]
fn w305_is_an_error_under_gsd_strict() {
    let p = with(&third_plan("", "\"src/tasks.js\"", ""));
    strict_fails(&p, "W305");
}

#[test]
fn an_action_for_now_static_is_e312() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "action \"Write src/tasks.js with add() and list()\"",
        "action \"Write src/tasks.js with a static for now list()\"",
    );
    assert_eq!(p.codes(), ["gsd/E312"]);
    assert!(
        only(&p, "gsd/E312")
            .contains("task 'v1.0/01/01-01/T1' reduces scope: its action says \"static for now\"")
    );
}

/// Plan 01-02 delivers TASK-01 instead of TASK-02: the requirement of phase 01 that no plan cites is the one gap.
fn uncovered_requirement() -> Project {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  requirements   [TASK-02]\n",
        "  requirements   [TASK-01]\n",
    );
    p
}

#[test]
fn an_uncovered_phase_requirement_is_w317() {
    let p = uncovered_requirement();
    // The same gap is E301 (no plan names it) and W317 (no plan text cites it).
    assert_eq!(p.codes(), ["gsd/E301", "gsd/W317"]);
    assert_eq!(
        message(&p, "gsd/W317"),
        "⚠ 1 of 3 items not covered by any plan: TASK-02"
    );
}

#[test]
fn post_planning_gaps_off_silences_w317() {
    let p = uncovered_requirement();
    p.settings(serde_json::json!({ "workflow.post_planning_gaps": false }));
    assert_eq!(p.codes(), ["gsd/E301"], "only the gap report is off");
}

#[test]
fn an_unplanned_phase_is_not_w317() {
    let p = project("plan");
    p.write(
        "spec/third.spec",
        "requirement SYNC-01 \"Sync\" {\n  text \"Tasks sync\"\n  category \"Sync\"\n  tier v1\n}\n\nphase 03 \"Sync\" {\n  milestone v1.0\n  name \"Sync\"\n  slug \"sync\"\n  goal \"Tasks sync between machines\"\n  depends_on [02]\n  requirements [SYNC-01]\n  success_criteria [\"a\", \"b\"]\n}\n",
    );
    assert_eq!(p.codes(), CLEAN);
}

fn phase_1_decision(text: &str) -> String {
    format!(
        "decision D-02 \"Another\" {{\n  phase  v1.0/01\n  area   \"Storage\"\n  text   \"{text}\"\n  origin context\n}}\n"
    )
}

#[test]
fn phase_requirement_in_no_plan_is_e301() {
    // Plan 01-02 delivers TASK-01 instead of TASK-02: phase 01 names TASK-02 and no plan does.
    let p = uncovered_requirement();
    assert_eq!(p.codes(), ["gsd/E301", "gsd/W317"]);
    assert_eq!(
        message(&p, "gsd/E301"),
        "requirement 'TASK-02' of phase 'v1.0/01' is in no plan of the phase"
    );
}

#[test]
fn an_unplanned_phase_is_not_e301() {
    // The unplanned phase 03 of `an_unplanned_phase_is_not_w317` names a requirement and has no plan.
    let p = project("plan");
    p.write(
        "spec/third.spec",
        "requirement SYNC-01 \"Sync\" {\n  text \"Tasks sync\"\n  category \"Sync\"\n  tier v1\n}\n\nphase 03 \"Sync\" {\n  milestone v1.0\n  name \"Sync\"\n  slug \"sync\"\n  goal \"Tasks sync between machines\"\n  depends_on [02]\n  requirements [SYNC-01]\n  success_criteria [\"a\", \"b\"]\n}\n\ncontext 03-CONTEXT \"Sync context\" {\n  phase v1.0/03\n  gathered 2026-10-28\n  boundary \"Sync\"\n  canonical_refs [{ topic \"No external specs: none\" paths [] }]\n}\n\ndecision D-01 \"Sync by file\" {\n  phase v1.0/03\n  text \"Tasks sync through one shared file\"\n  origin context\n}\n",
    );
    assert_eq!(p.codes(), CLEAN, "an unplanned phase has nothing to cover");
}

#[test]
fn an_uncited_context_decision_is_e310() {
    let p = with(&phase_1_decision("Completed tasks move to a done list"));
    // GSD's gap report also counts the decision: it is in no plan text.
    assert_eq!(p.codes(), ["gsd/E310", "gsd/W317"]);
    assert_eq!(
        message(&p, "gsd/E310"),
        "decision 'v1.0/01/D-02' is cited by no plan of phase 'v1.0/01'"
    );
    let found = p.findings();
    assert!(
        found
            .iter()
            .any(|(c, m)| c == "gsd/W317" && m == "⚠ 1 of 4 items not covered by any plan: D-02"),
        "{found:#?}"
    );
}

#[test]
fn a_decision_cited_by_its_first_six_words_is_covered() {
    // No `D-02` anywhere, but a task action repeats the decision's first six words; W317 counts only the id.
    let p = with(&phase_1_decision("Completed tasks move to a done list"));
    p.edit(
        "spec/plans.spec",
        "action \"Write src/complete.js with complete(id)\"",
        "action \"Completed tasks move to a done list: write src/complete.js\"",
    );
    assert_eq!(p.codes(), ["gsd/W317"]);
}

#[test]
fn a_project_decision_is_not_tracked() {
    // The project's `D-01` is cited by no plan and is clean; so is a decision of another origin.
    let p = with(
        "decision D-03 \"A summary decision\" {\n  phase  v1.0/01\n  text   \"Written by a summary\"\n  origin summary\n}\n",
    );
    assert_eq!(p.codes(), CLEAN);
}

#[test]
fn context_coverage_gate_off_silences_e310() {
    let p = with(&phase_1_decision("Completed tasks move to a done list"));
    p.settings(serde_json::json!({ "workflow.context_coverage_gate": false }));
    assert_eq!(p.codes(), ["gsd/W317"]);
}

#[test]
fn a_justified_coupling_is_i321() {
    // A third plan of wave 1 couples with 01-01 and says why.
    let p = with(&third_plan(
        "  coupling_justified [{ peer 01-01 reason \"both read the task list\" }]\n",
        "\"src/third.js\"",
        "",
    ));
    assert_eq!(p.codes(), ["gsd/I321"]);
    assert_eq!(
        message(&p, "gsd/I321"),
        "coupling between 'v1.0/01/01-03' and 'v1.0/01/01-01' is justified: both read the task list"
    );
}

#[test]
fn a_coupling_peer_in_another_wave_is_i321() {
    // 01-02 is wave 2: naming it from a wave-1 plan exempts nothing.
    let p = with(&third_plan(
        "  coupling_justified [{ peer 01-02 reason \"both read the task list\" }]\n",
        "\"src/third.js\"",
        "",
    ));
    assert_eq!(p.codes(), ["gsd/I321"]);
    assert!(message(&p, "gsd/I321").contains("is not in its wave: the exemption covers nothing"));
}
