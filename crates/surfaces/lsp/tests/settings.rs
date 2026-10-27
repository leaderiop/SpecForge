//! The language server shows what the project's report shows (ADR 0288).

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "the language server publishes a raised severity and no gated code"
)]
#[ignore = "K6 T6"]
fn the_lsp_publishes_raised_severities() {
    // The published severity of W902 is ERROR under the strict profile.
    unimplemented!("K6 T6: LSP publish");
}
