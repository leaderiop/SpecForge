//! A computed field in the language server (ADR 0287).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "compute_field_values",
    verify = "hover marks a computed field and completion offers none"
)]
#[ignore = "K4 T5"]
fn hover_marks_a_computed_field_and_completion_offers_none() {
    // hover shows - `status` = passed *(computed)*; field completion lists no status
    unimplemented!(
        "K4 T5: hover shows - `status` = passed *(computed)*; field completion lists no status"
    );
}
