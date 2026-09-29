//! Its own test binary: the guard this test records must not race the
//! registry assertions in `macro_test.rs`.

/// The attribute alone registers the test: there is no `#[test]` here, yet
/// this runs (and shows in `cargo test -- --list`).
#[specforge_test_macros::test(
    behavior = "record_test_via_drop_guard",
    verify = "the attribute registers the test without #[test]"
)]
fn the_attribute_alone_registers_the_test() {
    assert_eq!(1 + 1, 2);
}
