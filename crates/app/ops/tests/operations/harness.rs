//! A host write keeps the harness ledger, and refuses what a field's writer forbids (ADR 0287).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "refuse_written_computed",
    verify = "a typed edit of a computed, harness or immutable field is refused"
)]
#[ignore = "K4 T8"]
fn a_typed_edit_of_a_computed_harness_or_immutable_field_is_refused() {
    // set status passed is E080 (E082); set result pass E080 (E083); set origin on a committed decision E080 (E084)
    unimplemented!(
        "K4 T8: set status passed is E080 (E082); set result pass E080 (E083); set origin on a committed decision E080 (E084)"
    );
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
