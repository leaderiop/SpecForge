//! The value bytes a ledger digest, a history check and `previous` share (ADR 0287 U2).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "record_harness_values",
    verify = "the value digest reads the target of a reference, not its spelling"
)]
#[ignore = "K4 T6"]
fn the_value_digest_reads_the_target_not_the_spelling() {
    // `phase 03` and `phase v1.0/03` name one target and give equal `wire::bytes`; block keys in any order give
    // equal bytes; an unresolved reference digests its text. Written against `specforge_graph::wire` in T6.
    unimplemented!("K4 T6: specforge_graph::wire::{{resolved, bytes}}");
}
