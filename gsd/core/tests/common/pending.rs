//! Upstream paths of gsd-core v1.19.0 whose SpecForge port has not been redone yet (plan 04). Each port test skips a
//! listed path; `pending_resync_is_empty` (ignored until plan 06 T5) requires the list to be empty.
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

pub const PENDING_RESYNC: &[&str] = &["tests/gsd-secret-read-guard.test.cjs"];

pub fn is_pending(path: &str) -> bool {
    PENDING_RESYNC.contains(&path)
}

/// For a listed path, runs its check and requires it to fail, so the list cannot hold a path whose port is already
/// redone. `check` is the reader's own assertion for that path (it panics on a difference).
pub fn assert_still_pending(path: &str, check: impl FnOnce()) {
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check)).is_err();
    assert!(
        failed,
        "{path} is in PENDING_RESYNC but its port already matches gsd-core v{}: take it off the list",
        gsd_kit::tracked_version!()
    );
}
