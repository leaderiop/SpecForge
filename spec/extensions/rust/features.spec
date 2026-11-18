requirement FEATURE-116 "Rust Test Collection" {
  category "Feature"
  tier     v2
  text     """
    Rust test frameworks (cargo test, nextest) produce results in various
    formats, but none links a test to the spec entity it proves, so
    coverage could count obligations but never proof.

    Tests name the entity they prove with the #[specforge_test] attribute,
    which records each result to a per-binary report. @specforge/cargo-test
    declares the command `specforge collect` runs (cargo test) and maps
    those reports to entities (ADR 0002); the explicit attribute takes
    precedence over naming conventions.
  """
}

requirement FEATURE-115 "Rust Proc Macro Annotation" {
  category "Feature"
  tier     v2
  text     """
    Naming conventions alone are fragile and can break when test functions
    are renamed. Developers need explicit, compiler-checked linkage from
    test functions to spec entity IDs.

    The #[specforge_test(behavior = "entity_id", verify = "...")] proc
    macro attribute registers the test and wraps its body with a
    Drop-based guard that records pass/fail. Results are written to
    target/specforge/ for `specforge collect` (@specforge/cargo-test).
    Composable with #[tokio::test], #[rstest], etc., which keep
    registering their tests.
  """
}
