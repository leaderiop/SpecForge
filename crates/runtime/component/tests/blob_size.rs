//! The committed builtins are built small (`[profile.blob]`, I1): a total budget so a build
//! that drops the profile shows up as a test failure, not as a bigger binary and pack. The cap on one blob is the gate's
//! `blob-size` check (ADR 0338 D8).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::builtins::BUILTINS;
use specforge_component::committed::COMMITTED;

// The twelve builtins total 9,255,648 B, the sum of the `.wasm` files `revendor` commits (built with
// `--profile blob`); `@specforge/checks` is the largest, 2,381,815 B. The budget is that total rounded up to the next
// 100,000 B, here 9,300,000 (44,352 B of headroom): a change that grows the blobs past it re-measures and takes the
// rule's next step in the same commit. A build without the profile is far larger, so this catches a profile regression.
const TOTAL_BUDGET: usize = 9_300_000;

#[test]
fn committed_builtins_stay_under_their_size_budget() {
    let sizes: Vec<(&str, usize)> = COMMITTED[..BUILTINS.len()]
        .iter()
        .map(|b| (b.name, b.read().len()))
        .collect();
    let total: usize = sizes.iter().map(|(_, n)| n).sum();
    assert!(total <= TOTAL_BUDGET, "builtins total {total} B: {sizes:?}");
}
