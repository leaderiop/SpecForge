//! A setting name is a dotted name or a family whose `*` segments each stand for one key (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "a family matches its keys, closed or open, and an exact name beats it"
)]
#[ignore = "K6 T3"]
fn a_family_matches_its_keys() {
    // `models.*` over closed keys; `tiers.*.*`; `review.models.kimi-code` (a `-` in a segment); an exact name wins
    // over a family that also matches.
    unimplemented!("K6 T3: Family::matches");
}
