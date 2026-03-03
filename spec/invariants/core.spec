// Core compiler invariants — guarantees that must always hold

invariant spec_root_singleton "Spec Root Singleton" {
  guarantee """
    Exactly one spec block MUST exist across all .spec files in a project.
    The compiler MUST reject any project with zero or more than one spec block.
  """
  enforced_by [parse_spec_file_to_ast, link_entity_references]
  risk medium

  verify property "a project with exactly one spec block is accepted"
  verify unit "a project with zero or two spec blocks produces a diagnostic"

  tests ["../crates/specforge-validator/src/passes.rs"]
}

invariant multi_error_collection "Multi-Error Collection" {
  guarantee """
    The compiler MUST collect and report all diagnostics found during a
    compilation pass. It MUST NOT halt on the first error. Every error,
    warning, and info diagnostic MUST be emitted to the user.
  """
  enforced_by [detect_dangling_references, format_diagnostics_with_source_context]
  risk high

  verify property "a file with N errors produces exactly N error diagnostics in one pass"
  verify unit "the compiler does not halt after the first error"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant string_interning_consistency "String Interning Consistency" {
  guarantee """
    All interned strings MUST resolve to the same symbol for the same input
    within a compilation session. Two equal strings MUST produce the same
    interned key. Comparison by interned key MUST be equivalent to
    comparison by string value.
  """
  enforced_by [parse_spec_file_to_ast]
  risk high

  verify property "interning the same string twice returns the same key"
  verify unit "comparison by interned key is equivalent to comparison by string value"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

invariant import_dag "Import DAG" {
  guarantee """
    The use import graph MUST form a directed acyclic graph (DAG).
    The compiler MUST detect and reject circular imports with an E003
    diagnostic that names the cycle participants.
  """
  enforced_by [detect_import_cycles]
  risk medium

  verify property "an acyclic import graph is accepted without diagnostics"
  verify unit "a circular import produces E003 naming the cycle participants"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

invariant entity_id_uniqueness "Entity ID Uniqueness" {
  guarantee """
    Every entity ID MUST be globally unique across all .spec files in a project.
    The compiler MUST reject duplicate IDs with an E002 diagnostic that
    names both declaration sites.
  """
  enforced_by [detect_duplicate_entity_ids]
  risk high

  verify property "all unique entity IDs across files are accepted"
  verify unit "a duplicate entity ID produces E002 naming both declaration sites"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

invariant library_dag "Library DAG" {
  guarantee """
    The depends_on edges between library nodes MUST form a directed acyclic
    graph. The compiler MUST detect and reject circular library dependencies
    with an E007 diagnostic.
  """
  enforced_by [detect_library_cycles]
  risk medium

  verify property "an acyclic library dependency graph is accepted"
  verify unit "a circular library dependency produces E007"

  tests ["../crates/specforge-cli/tests/integration_test.rs", "../crates/specforge-validator/src/passes.rs"]
}

invariant incremental_correctness "Incremental Correctness" {
  guarantee """
    After an incremental recompilation triggered by a file change, the
    in-memory graph MUST be identical to the graph produced by a full
    cold rebuild of the same source files. No stale nodes or edges
    MUST remain from the previous compilation.
  """
  enforced_by [watch_file_system_for_changes, rebuild_affected_subgraph]
  risk high

  verify property "incremental recompilation produces the same graph as a full rebuild"
  verify unit "no stale nodes or edges remain after incremental recompilation"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
