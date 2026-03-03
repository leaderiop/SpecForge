// Resolution behaviors — import resolution and symbol linking

use invariants/core
use invariants/validation
use types/core
use types/graph
use types/errors
use ports/outbound

behavior resolve_use_imports "Resolve Use Imports" {
  invariants [import_dag, reference_resolution_completeness]
  types      [SpecFile, FileEntry]
  ports      [FileSystem]

  contract """
    The resolver MUST process use directives by locating the target .spec
    file relative to the spec root directory. The .spec extension MUST be
    appended implicitly. Missing files MUST produce an E001 diagnostic.
    The resolver MUST build the file dependency graph from imports.
  """

  verify unit        "resolve use path to file on disk"
  verify unit        "missing import file produces E001"
  verify integration "imports across nested directories resolve correctly"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior detect_import_cycles "Detect Import Cycles" {
  invariants [import_dag]
  types      [CycleError]

  contract """
    The resolver MUST detect circular use imports using topological sort.
    When a cycle is detected, the compiler MUST emit an E003 diagnostic
    that names all files participating in the cycle. Files involved in
    a cycle MUST NOT prevent processing of non-cyclic files.
  """

  verify unit "detect direct cycle between two files"
  verify unit "detect transitive cycle across three files"
  verify unit "non-cyclic files still process when a cycle exists"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior link_entity_references "Link Entity References" {
  invariants [string_interning_consistency, entity_id_uniqueness, reference_resolution_completeness]
  types      [EntityId, Graph, Edge, ResolutionError]

  contract """
    The resolver MUST link every entity ID in a reference list to the
    declaration site of that entity. Linked references MUST create edges
    in the in-memory graph. Unresolvable references MUST produce an E001
    diagnostic with a "did you mean?" suggestion when a close match exists.
  """

  verify unit "reference list IDs create graph edges"
  verify unit "unresolvable reference produces E001"
  verify unit "close match triggers did-you-mean suggestion"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior resolve_soft_cross_plugin_references "Resolve Soft Cross-Plugin References" {
  invariants [reference_resolution_completeness]
  types      [EntityId]

  contract """
    When a reference uses an entity type owned by a plugin that is not
    installed, the resolver MUST NOT emit E001. Instead it MUST emit
    an I004 info diagnostic suggesting the plugin to install.
    When the plugin IS installed, normal E001 validation MUST apply.
  """

  verify unit "uninstalled plugin entity type emits I004 not E001"
  verify unit "installed plugin entity type emits E001 on miss"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}

behavior resolve_external_ref_declarations "Resolve External Ref Declarations" {
  invariants [reference_resolution_completeness]
  types      [EntityId, Graph]
  ports      [FileSystem]

  contract """
    The resolver MUST process ref entity declarations, registering each
    ref as a node in the in-memory graph with its scheme and identifier.
    Refs with schemes that match an installed provider MUST be marked for
    provider validation. Refs with unrecognized schemes MUST be deferred
    and emit I005 if no provider is installed for the scheme.
  """

  verify unit "ref with known scheme is registered and marked for provider validation"
  verify unit "ref with unknown scheme emits I005"
  verify unit "ref node is added to graph with scheme metadata"

  tests ["../crates/specforge-cli/tests/integration_test.rs"]
}
