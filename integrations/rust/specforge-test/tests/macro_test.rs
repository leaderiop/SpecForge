use specforge_test::registry::{self, TestOutcome};
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn macro_injects_guard_that_records_to_registry() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    registry::drain();

    annotated_passing_test();

    let entries = registry::drain();
    assert_eq!(entries.len(), 1, "expected 1 entry from macro-annotated function");
    assert_eq!(entries[0].entity_kind, "behavior");
    assert_eq!(entries[0].entity_id, "create_user");
    assert_eq!(entries[0].outcome, TestOutcome::Pass);
}

#[test]
fn macro_supports_invariant_kind() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    registry::drain();

    annotated_invariant_test();

    let entries = registry::drain();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entity_kind, "invariant");
    assert_eq!(entries[0].entity_id, "unique_ids");
}

#[test]
fn macro_accepts_custom_entity_kind() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    registry::drain();

    annotated_custom_kind_test();

    let entries = registry::drain();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entity_kind, "constraint");
    assert_eq!(entries[0].entity_id, "naming_convention");
    assert_eq!(entries[0].outcome, TestOutcome::Pass);
}

#[test]
fn multiple_attributes_produce_multiple_guards() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    registry::drain();

    annotated_multi_entity();

    let entries = registry::drain();
    assert_eq!(entries.len(), 2, "expected 2 entries from double-annotated function");
    let kinds: Vec<&str> = entries.iter().map(|e| e.entity_kind.as_str()).collect();
    assert!(kinds.contains(&"behavior"));
    assert!(kinds.contains(&"invariant"));
}

#[test]
fn panicking_annotated_function_records_fail() {
    let _lock = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    registry::drain();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        annotated_panicking();
    }));

    assert!(result.is_err());
    let entries = registry::drain();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].outcome, TestOutcome::Fail);
}

// Helper functions with the macro attribute (NOT #[test] — called directly)
#[specforge_test_macros::test(behavior = "create_user")]
fn annotated_passing_test() {
}

#[specforge_test_macros::test(invariant = "unique_ids")]
fn annotated_invariant_test() {
}

#[specforge_test_macros::test(constraint = "naming_convention")]
fn annotated_custom_kind_test() {
}

#[specforge_test_macros::test(behavior = "create_user")]
#[specforge_test_macros::test(invariant = "unique_ids")]
fn annotated_multi_entity() {
}

#[specforge_test_macros::test(behavior = "delete_user")]
fn annotated_panicking() {
    panic!("simulated failure");
}
