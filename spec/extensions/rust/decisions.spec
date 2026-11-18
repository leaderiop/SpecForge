decision D-010 "Drop-Based Test Guard" {
  settled true
  origin  project
  text    """
    Use a Drop-based TestGuard. On creation, record entity ID and test name.
    On drop, check std::thread::panicking() — false means pass, true means
    fail, inverted under #[should_panic] (the attribute reads the
    expectation). Write results to target/specforge/ via atexit handler.
  """
}

decision D-009 "Double Underscore Entity Separator" {
  settled true
  origin  project
  text    """
    Use double underscore (__) as the separator between entity ID and
    description slug: {entity_id}__{description_slug}. Example:
    validate_input__rejects_empty_name. The entity ID is everything
    before the first __.
  """
}

decision D-089 "Phased Rust Delivery" {
  settled true
  origin  project
  text    """
    Three-phase delivery: Phase 1 ships test collection with
    convention-based mapping (zero Rust dependencies). Phase 2 ships the
    proc macro crate for explicit annotation. Phase 3 adds advanced
    features (watch, LSP, suspect links).
  """
}
