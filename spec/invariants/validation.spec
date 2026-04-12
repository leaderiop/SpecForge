// Validation-specific invariants

invariant reference_resolution_completeness "Reference Resolution Completeness" {
  guarantee """
    Every entity ID in a reference list MUST resolve to a declared entity.
    The compiler MUST emit E001 for unresolvable hard references and I004
    for unresolvable soft references (cross-extension). No reference MUST be
    silently ignored.
  """
  risk high

  verify property "every entity ID in a reference list resolves to a declared entity or emits a diagnostic"
  verify unit "E001 is emitted for broken hard references and I004 for broken soft references"

}

invariant diagnostic_determinism "Diagnostic Determinism" {
  guarantee """
    Given identical .spec source files, the compiler MUST produce identical
    diagnostics in the same order. No diagnostic MUST depend on filesystem
    iteration order, hashmap ordering, or wall-clock time.
  """
  risk medium

  verify property "identical source files produce identical diagnostics in the same order"
  verify unit "diagnostic output does not depend on filesystem iteration order or hashmap ordering"

}

// testable_entity_classification is defined in invariants/zero-entity-core.spec
// (merged from here to eliminate E002 duplicate entity ID)

invariant validation_pipeline_ordering "Validation Phase Ordering" {
  guarantee """
    All structural validators and all declarative validators MUST complete
    before aggregate_diagnostic_summary fires. Both validator categories
    execute after graph_built. The ordering between structural and declarative
    validators is not mandated — they MAY execute concurrently as co-consumers
    of graph_built.
  """
  risk medium

  verify integration "all validators complete before aggregate_diagnostic_summary fires"
  verify property "no diagnostic reaches aggregate_diagnostic_summary before both validator categories complete"
}

invariant diagnostic_code_uniqueness "Diagnostic Code Uniqueness" {
  guarantee """
    Each diagnostic code (E###, W###, I###) MUST have exactly one owner
    (core or extension). No two validation rules may emit the same code.
    Allocation ranges are partitioned between core and extensions.
  """
  risk high

  verify property "Diagnostic Code Uniqueness guarantee holds"
}
