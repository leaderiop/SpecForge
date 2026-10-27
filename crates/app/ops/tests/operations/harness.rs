//! A host write keeps the harness ledger, and refuses what a field's writer forbids (ADR 0287).

use std::sync::Arc;

use serde_json::json;
use specforge_ops::authoring::edit::{self, Input};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    FakeHistory, GSD_EXTENSION, TestProject, gsd_history_runtime, history,
};
use specforge_test_macros::test as specforge_test;

const BASE: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\n";

fn refused(project: &TestProject, entity: &str, field: &str, value: &str) -> bool {
    let compiled = project.compile_in(Arc::new(gsd_history_runtime(json!([]))));
    edit::run(
        &ProjectView::of(&compiled),
        &[crate::edit::set(entity, field, Input::One(value.into()))],
        true,
        specforge_ops::testing::caller(),
    )
    .is_err_and(|e| e.error.is(specforge_common::codes::E080))
}

fn gsd(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file("a.spec", format!("{BASE}{spec}"))
}

#[specforge_test(
    behavior = "refuse_written_computed",
    verify = "a typed edit of a computed, harness or immutable field is refused"
)]
fn a_typed_edit_of_a_computed_harness_or_immutable_field_is_refused() {
    let verification = gsd("verification V \"V\" {\n  phase 03\n}\n");
    assert!(refused(&verification, "v1.0/03/V", "status", "passed"));
    let uat = gsd("uat_test U \"U\" {\n  phase 03\n}\n");
    assert!(refused(&uat, "v1.0/03/U", "result", "pass"));
    let decision = "decision D \"D\" {\n  phase 03\n  origin context\n}\n";
    let committed = gsd(decision);
    let fake = Arc::new(FakeHistory::repo(
        committed.root(),
        &"a".repeat(40),
        vec![("a.spec".into(), format!("{BASE}{decision}").into_bytes())],
    ));
    let _installed = history::install(fake);
    assert!(refused(&committed, "v1.0/03/D", "origin", "project"));
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "a rename records its move and re-keys its lines"
)]
#[ignore = "K4 T13"]
fn a_rename_records_its_move_and_rekeys_its_lines() {
    // rename v1.0/03 05: ledger gains v1.0/05 moved:v1.0/03, U1 result re-keyed, no E083
    unimplemented!(
        "K4 T13: rename v1.0/03 05: ledger gains v1.0/05 moved:v1.0/03, U1 result re-keyed, no E083"
    );
}

#[specforge_test(
    behavior = "record_harness_values",
    verify = "two writers both land their lines"
)]
#[ignore = "K4 T13"]
fn two_writers_both_land_their_lines() {
    // two threads apply plans on disjoint files, both ledger lines present
    unimplemented!("K4 T13: two threads apply plans on disjoint files, both ledger lines present");
}

#[specforge_test(
    behavior = "collect_harness_ledger",
    verify = "the first ledger write adds the union attribute"
)]
#[ignore = "K4 T14"]
fn the_first_ledger_write_adds_the_union_attribute() {
    // init and the first ledger write each add /specforge-harness merge=union once
    unimplemented!(
        "K4 T14: init and the first ledger write each add /specforge-harness merge=union once"
    );
}
