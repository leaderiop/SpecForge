requirement INVARIANT-117 "Reference Resolution Completeness" {
  category "Invariant"
  tier     v2
  text     """
    Every entity ID in a reference list MUST resolve to a declared entity.
    The compiler MUST emit E003 for unresolvable hard references and I004
    for unresolvable soft references (cross-extension). No reference MUST be
    silently ignored.
  """
}

requirement INVARIANT-018 "Diagnostic Determinism" {
  category "Invariant"
  tier     v2
  text     """
    Given identical .spec source files, the compiler MUST produce identical
    diagnostics in the same order. No diagnostic MUST depend on filesystem
    iteration order, hashmap ordering, or wall-clock time.
  """
}

requirement INVARIANT-145 "Validation Phase Ordering" {
  category "Invariant"
  tier     v2
  text     """
    All structural validators and all declarative validators MUST complete
    before aggregate_diagnostic_summary fires. Both validator categories
    execute after graph_built. The ordering between structural and declarative
    validators is not mandated — they MAY execute concurrently as co-consumers
    of graph_built.
  """
}

requirement INVARIANT-017 "Diagnostic Code Uniqueness" {
  category "Invariant"
  tier     v2
  text     """
    Each diagnostic code (E###, W###, I###, A###, and the registry
    client's R### and R-<AREA>-###) MUST have exactly one meaning, one
    owner (core or one extension) and one level, all stated once in the
    diagnostic catalog. Several rules of the owning extension MAY share
    a code (one per target kind). The host builds a diagnostic only from
    a core code's constant, at the code's level; only a diagnostic policy
    changes a severity afterwards. A code an extension reports MUST be
    its own catalogued code at the level its letter states, qualified by
    its namespace; any other is reported (W150).
  """
}
