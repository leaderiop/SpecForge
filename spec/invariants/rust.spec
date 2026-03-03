// Rust-specific invariants

invariant deterministic_rust_generation "Deterministic Rust Generation" {
  guarantee """
    Given identical .spec source files and identical RustGenConfig,
    specforge gen rust MUST produce byte-identical output files. SHA256
    checksums MUST match across runs on any platform. No output MUST
    depend on filesystem ordering, timestamps, or random values.
  """
  enforced_by [generate_rust_structs_from_types, detect_rust_code_drift]
  risk high

  verify property "identical spec + config produces byte-identical Rust output"
  verify unit "checksums match across multiple runs"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant entity_mapping_precedence "Entity Mapping Precedence" {
  guarantee """
    Test-to-entity resolution MUST follow strict precedence: tests field
    (1st) > proc macro attribute (2nd) > naming convention (3rd). A higher
    level MUST always override a lower level. No ambiguous mappings MUST
    exist after resolution — conflicts MUST produce diagnostics.
  """
  enforced_by [collect_rust_test_results, resolve_entity_mapping]
  risk high

  verify property "tests field always overrides proc macro and convention"
  verify unit "ambiguous mappings produce diagnostics"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant rust_drift_detection_accuracy "Rust Drift Detection Accuracy" {
  guarantee """
    specforge gen rust --check MUST detect every case where regeneration
    would produce different output. Zero false negatives: no stale file
    MUST pass drift detection. Zero false positives: no current file
    MUST be flagged as stale.
  """
  enforced_by [detect_rust_code_drift, safe_rust_regeneration]
  risk medium

  verify property "every stale file is detected by drift check"
  verify unit "current files are not flagged as stale"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
