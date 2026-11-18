requirement FEATURE-015 "cargo test Collection" {
  category "Feature"
  tier     v2
  text     """
    Rust tests prove entities through the `specforge-test` attribute, but
    getting their results into coverage meant running `cargo test` in
    exactly the way SpecForge expected and pointing `collect` at the
    output by hand.

    @specforge/cargo-test is the Rust runner extension (ADR 0002). It
    declares the command and report location, so `specforge collect` runs
    `cargo test` with the user's consent and hands the per-binary reports
    to the extension's pure mapping.
  """
}
