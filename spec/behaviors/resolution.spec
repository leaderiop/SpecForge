// Resolution behaviors — import resolution and symbol linking

use invariants/core
use invariants/validation
use types/core
use types/graph
use types/errors
use ports/outbound
use events/compilation

behavior resolve_use_imports "Resolve Use Imports" {
  invariants [import_dag, reference_resolution_completeness]
  types      [SpecFile, FileEntry]
  ports      [FileSystem]
  consumes  [registries_populated]

  contract """
    The resolver MUST process use directives by locating the target .spec
    file relative to the spec root directory. The .spec extension MUST be
    appended implicitly. Missing files MUST produce an E025 diagnostic.
    The resolver MUST build the file dependency graph from imports.
  """

  verify unit        "resolve use path to file on disk"
  verify unit        "missing import file produces E025"
  verify integration "imports across nested directories resolve correctly"

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

}

behavior link_entity_references "Link Entity References" {
  invariants [string_interning_consistency, entity_id_uniqueness, reference_resolution_completeness]
  types      [EntityId, Graph, Edge, ResolutionError]
  produces   [resolution_complete]

  contract """
    The resolver MUST link every entity ID in a reference list to the
    declaration site of that entity. Each resolved reference MUST be
    recorded as a pending edge (source, target, edge type) for the
    graph builder to materialize. Unresolvable references MUST produce
    an E001 diagnostic with a "did you mean?" suggestion when a close
    match exists.
  """

  verify unit "reference list IDs create graph edges"
  verify unit "unresolvable reference produces E001"
  verify unit "close match triggers did-you-mean suggestion"

}

behavior resolve_soft_cross_extension_references "Resolve Soft Cross-Extension References" {
  invariants [reference_resolution_completeness]
  types      [EntityId]
  consumes   [registries_populated]

  contract """
    When an entity block uses a keyword not present in the KindRegistry,
    the compiler MUST consult the known-extensions catalog (a curated mapping
    of keywords to extensions) to determine if an installable extension provides
    that keyword. If a match is found, the compiler MUST emit an I004 info
    diagnostic suggesting the extension to install. If no match is found, the
    semantic validation phase MUST emit E024 via detect_unknown_entity_kinds.
    This is about entity declarations with unknown keywords, not reference
    list entries — reference lists contain entity IDs (not keywords), so the
    resolver cannot determine a target entity's kind from an unresolvable ID.
    When the extension IS installed and the target entity does not exist,
    normal E001 validation via link_entity_references MUST apply.
  """

  verify unit "unknown keyword matching known extension emits I004"
  verify unit "installed extension with missing entity emits E001"

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

}
