// Validation-specific invariants

invariant reference_resolution_completeness "Reference Resolution Completeness" {
  guarantee """
    Every entity ID in a reference list MUST resolve to a declared entity.
    The compiler MUST emit E003 for unresolvable hard references and I004
    for unresolvable soft references (cross-extension). No reference MUST be
    silently ignored.
  """
  risk      high
  verify property "every entity ID in a reference list resolves to a declared entity or emits a diagnostic"
  verify unit "E003 is emitted for broken hard references and I004 for broken soft references"
}

invariant diagnostic_determinism "Diagnostic Determinism" {
  guarantee """
    Given identical .spec source files, the compiler MUST produce identical
    diagnostics in the same order. No diagnostic MUST depend on filesystem
    iteration order, hashmap ordering, or wall-clock time.
  """
  risk      medium
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
  risk      medium
  verify integration "all validators complete before aggregate_diagnostic_summary fires"
  verify property "no diagnostic reaches aggregate_diagnostic_summary before both validator categories complete"
}

invariant diagnostic_code_uniqueness "Diagnostic Code Uniqueness" {
  guarantee """
    Each diagnostic code (E###, W###, I###, A###, and the registry
    client's R### and R-<AREA>-###) MUST have exactly one meaning, one
    owner (core or one extension) and one level, all stated once in the
    diagnostic catalog. Several rules of the owning extension MAY share
    a code (one per target kind). A diagnostic the host builds from a
    core code's constant has the code's level; only a diagnostic policy
    changes a severity afterwards. A code an extension reports MUST be
    its own catalogued code at its catalogued level, or a code in
    E900-E998, W900-W998 or I900-I998 whose prefix states its level;
    any other is reported (W150).
  """
  risk      high
  verify property "Diagnostic Code Uniqueness guarantee holds"
  verify unit "a core code's constant carries its catalogued level and owner"
  verify unit "a host diagnostic's severity is its code's catalogued level"
  verify unit "an extension reports only its own catalogued codes, or third-party codes whose prefix states their level"
}
