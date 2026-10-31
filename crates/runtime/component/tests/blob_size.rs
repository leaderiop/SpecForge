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
// 0.96 MB. Protocol 1.12.0 (human commands, `gates_when_on`, `cited_tests`, numbers) measured 4,799,354 B (K20a added
// 8,929 B). Protocol 1.13.0 (state and host accessors, the prompt tail, the served slash commands, `when` lists)
// measured 4,850,633 B: +51,279 B, about 5.7 KB per blob, not the 1 KB of a small minor. The untagged `when_names`
// decode in five descriptor types is the likely cost; it was not measured apart. The product blob is 981,363 B, 18,637 B
// under its cap, and the headroom under the total is 49,367 B. Protocol 1.14.0 (hooks: git facts, hosts, settings, the
// `Unknown` event, `git_when`) measured 4,874,602 B (+23,969 B, about 2.7 KB per blob). The product blob is 984,024 B,
// 15,976 B under its cap, which is the binding limit now. The rule (the measured total rounded up to the next 100,000)
// gives 4,900,000, with 25,398 B of headroom: the next minor will need to raise it. A build without the profile is
// about 7 MB, well over it, so this still catches a profile regression.
const TOTAL_BUDGET: usize = 4_900_000;
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
