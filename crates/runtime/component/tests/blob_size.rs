//! The committed builtins are built small (`[profile.blob]`, I1): a total budget so a build
//! that drops the profile shows up as a test failure, not as a bigger binary and pack. The cap on one blob is the gate's
//! `blob-size` check (ADR 0338 D8).
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use specforge_component::builtins::BUILTINS;
use specforge_component::committed::COMMITTED;

// The fifteen builtins total 17,798,221 B, the sum of the `.wasm` files `revendor` commits (built with
// `--profile blob` on the pinned rustc): the fourteen under `extensions/` 12,297,809 B, the largest of them
// `@specforge/checks` at 2,451,629 B, and `gsd/core/wasm/gsd_core.wasm` 5,500,412 B, the one blob over the gate's cap,
// held to its own allowance (ADR 0366 D3). The budget is that total rounded up to the next 100,000 B, here 17,800,000
// (1,779 B of headroom): a change that grows the blobs past it re-measures and takes the rule's next step in the same
// commit (ADR 0373 D3). A build without the profile is far larger, so this catches a profile regression.
const TOTAL_BUDGET: usize = 17_800_000;

#[test]
fn committed_builtins_stay_under_their_size_budget() {
    let sizes: Vec<(&str, usize)> = COMMITTED[..BUILTINS.len()]
        .iter()
        .map(|b| (b.name, b.read().len()))
        .collect();
    let total: usize = sizes.iter().map(|(_, n)| n).sum();
    assert!(total <= TOTAL_BUDGET, "builtins total {total} B: {sizes:?}");
}
