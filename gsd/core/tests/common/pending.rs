//! Upstream paths of the tracked gsd-core release whose SpecForge port has not been redone yet. Each port test skips a
//! listed path (and fails if its port already matches); `pending_resync_is_empty` requires the list to be empty. The
//! commit that moves the snapshot to a newer release lists the changed assets here and ignores that test; each re-port
//! takes its paths off, and the last one un-ignores it (`gsd/parity/README.md`, "To track a newer release").
#![allow(
    dead_code,
    reason = "each test crate that includes this module uses some of it"
)]

pub const PENDING_RESYNC: &[&str] = &[];

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
