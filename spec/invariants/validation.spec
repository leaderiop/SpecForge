// Validation-specific invariants

invariant reference_resolution_completeness "Reference Resolution Completeness" {
  guarantee """
    Every entity ID in a reference list MUST resolve to a declared entity.
    The compiler MUST emit E001 for unresolvable hard references and I004
    for unresolvable soft references (cross-plugin). No reference MUST be
    silently ignored.
  """
  enforced_by [link_entity_references, detect_dangling_references]
  risk high

  verify property "every entity ID in a reference list resolves to a declared entity or emits a diagnostic"
  verify unit "E001 is emitted for broken hard references and I004 for broken soft references"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

invariant rpn_arithmetic_integrity "RPN Arithmetic Integrity" {
  guarantee """
    When a failure_mode block declares severity, occurrence, detection, and
    rpn fields, the rpn value MUST equal severity * occurrence * detection.
    The same MUST hold for the post_mitigation sub-block. The compiler MUST
    emit E005 on mismatch.
  """
  enforced_by [validate_rpn_arithmetic]
  risk low

  verify property "rpn equals severity times occurrence times detection when all fields are present"
  verify unit "E005 is emitted when rpn does not match the arithmetic product"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

invariant diagnostic_determinism "Diagnostic Determinism" {
  guarantee """
    Given identical .spec source files, the compiler MUST produce identical
    diagnostics in the same order. No diagnostic MUST depend on filesystem
    iteration order, hashmap ordering, or wall-clock time.
  """
  enforced_by [deterministic_output, format_diagnostics_with_source_context]
  risk medium

  verify property "identical source files produce identical diagnostics in the same order"
  verify unit "diagnostic output does not depend on filesystem iteration order or hashmap ordering"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant traceability_chain_integrity "Traceability Chain Integrity" {
  guarantee """
    Every path in the tests field MUST reference an existing file (E016).
    Every entity ID in a specforge-report.json MUST match a declared entity
    in the spec graph. No broken links MUST exist in the intent → linkage → proof
    traceability chain. The compiler MUST detect and report all gaps.
  """
  enforced_by [validate_tests_field_references, consume_specforge_report]
  risk high

  verify property "every path in the tests field references an existing file"
  verify unit "broken traceability links in the intent-linkage-proof chain are detected and reported"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant testable_entity_classification "Testable Entity Classification" {
  guarantee """
    is_testable() and supports_scenario() MUST be consistent for both built-in
    and plugin entities. A testable entity MUST accept verify statements. Only
    behavior and capability entities MUST accept scenario blocks. Plugin entities
    marked as testable MUST have grammar support for verify/scenario.
  """
  enforced_by [validate_plugin_testability, load_plugin_manifests]
  risk medium

  verify unit "is_testable returns true for behavior, invariant, event, constraint, and capability"
  verify unit "supports_scenario returns true only for behavior and capability"

  tests ["../crates/specforge-validator/src/passes.rs"]
}
