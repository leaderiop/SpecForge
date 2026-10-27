//! Every surface shows a computed value as a field (ADR 0287).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "compute_field_values",
    verify = "every surface shows a computed value as a field and marks it computed"
)]
#[ignore = "K4 T5"]
fn every_surface_shows_a_computed_value_as_a_field() {
    // query and export nodes carry fields.status and computed [status]; inspect facts hold it
    unimplemented!(
        "K4 T5: query and export nodes carry fields.status and computed [status]; inspect facts hold it"
    );
}
