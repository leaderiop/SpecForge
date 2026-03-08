// Resolution behaviors — import resolution and symbol linking

use invariants/core
use invariants/validation
use invariants/zero-entity-core
use types/core
use types/graph
use types/errors
use ports/outbound
use events/compilation

behavior resolve_use_imports "Resolve Use Imports" {
  invariants [import_dag, reference_resolution_completeness, compilation_pipeline_ordering]
  types      [SpecFile, FileEntry]
  ports      [FileSystem]
  consumes  [registries_populated, define_blocks_registered]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming KindRegistry and FieldRegistry are ready"
    define_blocks_registered_fired "define_blocks_registered event has fired, confirming user-defined types are registered"
    filesystem_available "FileSystem port is available for locating .spec files on disk"
  }

  ensures {
    imports_resolved "All use directives are resolved to target .spec files relative to the spec root"
    missing_files_diagnosed "Missing import targets produce E025 diagnostics"
    dependency_graph_built "File dependency graph is constructed from resolved imports"
  }

  contract """
    The resolver MUST process use directives by locating the target .spec
    file relative to the spec root directory. The .spec extension MUST be
    appended implicitly. Missing files MUST produce an E025 diagnostic.
    The resolver MUST build the file dependency graph from imports.
  """

  verify unit        "resolve use path to file on disk"
  verify unit        "missing import file produces E025"
  verify integration "imports across nested directories resolve correctly"
  verify contract "requires/ensures consistency for use import resolution"

}

// No consumes — called inline during use import resolution
behavior detect_import_cycles "Detect Import Cycles" {
  invariants [import_dag]
  types      [CycleError, FileEntry]

  requires {
    import_graph_available "File dependency graph from use imports has been constructed"
  }

  ensures {
    cycles_detected "All circular import chains are identified via topological sort"
    cycle_diagnostic_emitted "Each detected cycle produces an E003 diagnostic naming all participating files"
    non_cyclic_unaffected "Files not involved in any cycle continue processing normally"
  }

  contract """
    The resolver MUST detect circular use imports using topological sort.
    When a cycle is detected, the compiler MUST emit an E003 diagnostic
    that names all files participating in the cycle. Files involved in
    a cycle MUST NOT prevent processing of non-cyclic files.
  """

  verify unit "detect direct cycle between two files"
  verify unit "detect transitive cycle across three files"
  verify unit "non-cyclic files still process when a cycle exists"
  verify contract "requires/ensures consistency for import cycle detection"

}

behavior link_entity_references "Link Entity References" {
  invariants [string_interning_consistency, entity_id_uniqueness, reference_resolution_completeness]
  types      [EntityId, Graph, Edge, ResolutionError]
  produces   [resolution_complete]

  requires {
    registries_populated "KindRegistry and FieldRegistry are populated from extension manifests"
    all_files_parsed "All .spec files have been parsed"
  }

  ensures {
    all_references_resolved "Every reference list entry either resolves to a declared entity or emits an E001 diagnostic"
    no_silent_ignoring "No reference is silently ignored"
  }

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
  verify contract "requires/ensures consistency for entity reference linking"

}

behavior resolve_soft_cross_extension_references "Resolve Soft Cross-Extension References" {
  invariants [reference_resolution_completeness]
  types      [EntityId]
  consumes   [registries_populated]

  requires {
    registries_populated_fired "registries_populated event has fired, confirming KindRegistry is available for keyword lookup"
    known_extensions_catalog_available "Known-extensions catalog (keyword-to-extension mapping) is loaded"
  }

  ensures {
    suggestion_emitted "Unknown keywords matching a known extension produce I004 info diagnostics"
    installed_extensions_resolved "Keywords from installed extensions follow normal E001 resolution"
  }

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
  verify unit "installed extension with imported file but missing entity emits E001"
  verify contract "requires/ensures consistency for soft cross-extension resolution"

}

// No consumes — called inline during reference resolution
behavior resolve_external_ref_declarations "Resolve External Ref Declarations" {
  invariants [reference_resolution_completeness]
  types      [EntityId, Graph]
  ports      [FileSystem]

  requires {
    ref_blocks_parsed "All ref blocks have been parsed with scheme, kind, and identifier extracted"
    filesystem_available "FileSystem port is available for provider validation"
  }

  ensures {
    refs_registered "Each ref declaration is registered as a node in the graph with scheme metadata"
    known_schemes_marked "Refs with schemes matching installed providers are marked for provider validation"
    unknown_schemes_deferred "Refs with unrecognized schemes emit I005 and are deferred"
  }

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
  verify contract "requires/ensures consistency for external ref resolution"

}
