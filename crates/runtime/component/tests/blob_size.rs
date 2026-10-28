//! The committed builtins are built small (`[profile.blob]`, I1): a size budget so a build
//! that drops the profile shows up as a test failure, not as a bigger binary and pack.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::builtins::BUILTINS;
use specforge_component::committed::COMMITTED;

// The blob profile built the nine at 3.3 MB; the 1.8.0 SDK (field forms, computed values, typed settings) adds ~70 KB to
// each (3.96 MB, product 0.86 MB); protocol 1.10.0 (host contributions, state, hooks) brings them to 4.71 MB, product
// 0.96 MB. A build without the profile is 7 MB, well over either.
const TOTAL_BUDGET: usize = 4_800_000;
const BLOB_BUDGET: usize = 1_000_000;

#[test]
fn committed_builtins_stay_under_their_size_budget() {
    let sizes: Vec<(&str, usize)> = COMMITTED[..BUILTINS.len()]
        .iter()
        .map(|b| (b.name, b.read().len()))
        .collect();
    let total: usize = sizes.iter().map(|(_, n)| n).sum();
    assert!(total <= TOTAL_BUDGET, "builtins total {total} B: {sizes:?}");
    for (name, n) in &sizes {
        assert!(*n <= BLOB_BUDGET, "{name} is {n} B");
    }
}
