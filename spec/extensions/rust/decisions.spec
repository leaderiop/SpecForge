// @specforge/rust extension Architecture Decision Records
//
// Decisions specific to the Rust language integration:
// test guard design, JUnit format, naming conventions, delivery phases.

use "extensions/rust/invariants"
decision drop_based_test_guard "Drop-Based Test Guard" {
  status   accepted
  date     2026-03-02

  context """
    The Rust proc macro needs to record whether a test passed or failed.
    Options: (1) wrap body in catch_unwind, (2) use a Drop-based guard
    that checks std::thread::panicking(). catch_unwind doesn't work with
    async tests, rstest, or proptest. Drop works universally except with
    #[should_panic].
  """

  decision """
    Use a Drop-based TestGuard. On creation, record entity ID and test name.
    On drop, check std::thread::panicking() — false means pass, true means
    fail. Write results to target/specforge/ via atexit handler.
    Document #[should_panic] as unsupported.
  """

  consequences [
    "Works with async tests, rstest, proptest, and all composable test attributes",
    "#[should_panic] tests are incompatible — records FAIL because panicking() is true during unwinding",
    "atexit handler introduces global state — acceptable for test binaries",
    "Simple implementation: ~50 lines of unsafe-free Rust",
  ]

  invariants [entity_mapping_precedence]
}

decision nextest_junit_primary_format "Nextest JUnit XML as Primary Format" {
  status   accepted
  date     2026-03-02

  context """
    Rust test output comes in multiple formats: cargo test text, libtest
    --format json (unstable since 2018), and nextest JUnit XML. The
    collect command needs a stable, machine-readable primary format.
  """

  decision """
    nextest JUnit XML is the primary format for specforge collect rust.
    It is stable, widely adopted, and contains sufficient metadata
    (classname, duration, failure messages). libtest JSON is supported
    behind a flag as secondary. cargo test text is best-effort.
  """

  consequences [
    "Stable format — no dependency on unstable compiler features",
    "nextest adoption is growing rapidly in Rust ecosystem",
    "JUnit XML is well-understood and parsed by many tools",
    "Users who don't use nextest can still use convention-based collection",
  ]
}

decision double_underscore_separator "Double Underscore Entity Separator" {
  status   accepted
  date     2026-03-02

  context """
    Convention-based entity mapping needs to extract entity IDs from test
    function names. Entity IDs use snake_case (single underscore). The
    separator must be unambiguous.
  """

  decision """
    Use double underscore (__) as the separator between entity ID and
    description slug: {entity_id}__{description_slug}. Example:
    validate_input__rejects_empty_name. The entity ID is everything
    before the first __.
  """

  consequences [
    "Unambiguous extraction — entity IDs never contain double underscores",
    "Human-readable: validate_input__rejects_empty translates clearly",
    "Compatible with Rust naming conventions",
    "Slightly unusual syntax, requires documentation",
  ]

  invariants [entity_mapping_precedence]
}

decision phased_rust_delivery "Phased Rust Delivery" {
  status   accepted
  date     2026-03-02

  context """
    Delivering everything at once is risky and delays all value. The extension
    has natural phase boundaries: convention-based (zero deps) -> proc macro
    -> advanced features.
  """

  decision """
    Three-phase delivery: Phase 1 ships specforge collect rust with
    convention-based mapping (zero Rust dependencies). Phase 2 ships the
    proc macro crate for explicit annotation. Phase 3 adds advanced
    features (watch, LSP, suspect links).
  """

  consequences [
    "Phase 1 delivers value immediately with no user-side dependencies",
    "Each phase adds a layer without breaking the previous",
    "Self-hosting (SpecForge testing itself) starts at Phase 1",
    "Phase boundaries align with crate release cadence",
  ]
}
