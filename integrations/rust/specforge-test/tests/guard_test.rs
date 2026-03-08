use specforge_test::guard::TestGuard;
use specforge_test::registry::{self, TestOutcome};
use std::panic;
use std::sync::Mutex;

// Guard tests must run serially because they share the global registry.
static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn passing_test_records_pass() {
    let _lock = SERIAL.lock().unwrap();
    registry::drain();

    {
        let _guard = TestGuard::new("behavior", "create_user", "mod", "test_fn", "test.rs", 1);
    }

    let entries = registry::drain();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entity_id, "create_user");
    assert_eq!(entries[0].entity_kind, "behavior");
    assert_eq!(entries[0].outcome, TestOutcome::Pass);
}

#[test]
fn panicking_test_records_fail() {
    let _lock = SERIAL.lock().unwrap();
    registry::drain();

    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let _guard = TestGuard::new("behavior", "delete_user", "mod", "test_fn", "test.rs", 1);
        panic!("simulated test failure");
    }));

    assert!(result.is_err());
    let entries = registry::drain();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entity_id, "delete_user");
    assert_eq!(entries[0].outcome, TestOutcome::Fail);
}

#[test]
fn multiple_guards_record_multiple_entries() {
    let _lock = SERIAL.lock().unwrap();
    registry::drain();

    {
        let _g1 = TestGuard::new("behavior", "create_user", "mod", "test_fn", "test.rs", 1);
        let _g2 = TestGuard::new("invariant", "unique_ids", "mod", "test_fn", "test.rs", 1);
    }

    let entries = registry::drain();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].entity_id, "unique_ids"); // dropped in reverse order
    assert_eq!(entries[1].entity_id, "create_user");
}

#[test]
fn drain_clears_registry() {
    let _lock = SERIAL.lock().unwrap();
    registry::drain();

    {
        let _guard = TestGuard::new("behavior", "foo", "mod", "test_fn", "test.rs", 1);
    }

    let first = registry::drain();
    assert_eq!(first.len(), 1);

    let second = registry::drain();
    assert_eq!(second.len(), 0);
}
