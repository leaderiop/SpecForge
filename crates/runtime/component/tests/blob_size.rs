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
// about 7 MB, well over it, so this still catches a profile regression. Protocol 1.15.0 (facts, `complete_when`,
// `result`) and the tenth builtin, `@code/core` (522,544 B with its ref and proof passes), measured 5,499,123 B; the
// nine grew 101,977 B together. R1 sets 5,900,000 (ADR 0328), which leaves 400,877 B of headroom: less than plan 03's
// `@javascript/node-test`, whose SDK floor alone is about 428 KB, so plan 03 must raise it or shrink a blob. The
// product blob is 998,381 B, 1,619 B under its cap: the next minor that grows every blob must raise the cap or shrink
// product. ADR 0330's `routes` (route, step and annotation descriptors, in every blob) measured 5,630,853 B: +131,730 B,
// about 13 KB per blob; the total keeps 269,147 B of headroom. The product blob is 1,011,905 B, so its cap takes the
// rule's next step, 1,100,000, with 88,095 B of headroom. The eleventh builtin, `@javascript/node-test` (524,429 B: the
// SDK, serde_json and the linked JUnit reader of `specforge-code-junit`), brings the total to 6,155,282 B, 96 KB more
// than the 428 KB SDK floor R1 planned for; the rule gives 6,200,000, with 44,718 B of headroom. Protocol 2.0.0's
// one list of inputs (an internally tagged `Input`, ADR 0337) measured 6,276,792 B; the rule gives 6,300,000, with
// 23,208 B of headroom. The product blob is 1,021,477 B, 78,523 B under its cap.
const TOTAL_BUDGET: usize = 6_300_000;
const BLOB_BUDGET: usize = 1_100_000;

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
