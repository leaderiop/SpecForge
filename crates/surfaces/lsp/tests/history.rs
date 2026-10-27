//! The language server and a commit (ADR 0287).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "watch_baseline_refs",
    verify = "the LSP republishes after a commit"
)]
#[ignore = "K4 T12"]
fn the_lsp_republishes_after_a_commit() {
    // didChangeWatchedFiles for the ref after a commit gives publishDiagnostics without the E084
    unimplemented!(
        "K4 T12: didChangeWatchedFiles for the ref after a commit gives publishDiagnostics without the E084"
    );
}
