// @specforge/cargo-test features

use "extensions/cargo-test/behaviors"

feature ct_cargo_test_collection "cargo test Collection" {
  behaviors [ct_declare_cargo_collector, ct_map_binary_reports]

  problem """
    Rust tests prove entities through the `specforge-test` attribute, but
    getting their results into coverage meant running `cargo test` in
    exactly the way SpecForge expected and pointing `collect` at the
    output by hand.
  """

  solution """
    @specforge/cargo-test is the Rust runner extension (ADR 0002). It
    declares the command and report location, so `specforge collect` runs
    `cargo test` with the user's consent and hands the per-binary reports
    to the extension's pure mapping.
  """
}
