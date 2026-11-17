requirement FEATURE-109 "Reference Resolution" {
  category "Feature"
  tier     v1
  text     """
    .spec files reference entities across multiple files via use imports
    and reference lists. Broken imports, circular dependencies, and
    unresolvable cross-extension references must be detected.

    The resolver processes use imports, links entity references, detects
    import cycles (W113), and resolves cross-extension references with soft
    resolution (I004 for uninstalled extensions). External ref declarations
    are resolved via provider schemes.
  """
}

requirement FEATURE-047 "Graph Construction" {
  category "Feature"
  tier     v1
  text     """
    After resolution, entities and their resolved references must be
    assembled into an in-memory typed entity graph for validation,
    export, and querying.

    The graph builder constructs an in-memory directed graph from
    resolved ASTs. Nodes represent entities with kind and fields. Edges
    represent resolved references with registered edge type labels.
  """
}

requirement FEATURE-124 "Structural Validation" {
  category "Feature"
  tier     v1
  text     """
    The compiled graph may contain structural inconsistencies: dangling
    references, duplicate IDs, unreferenced structural nodes, and missing
    file references.

    Core structural validation checks the graph for: dangling
    reference integrity (the linker's own debug assertion), duplicate
    IDs (E002) and unreferenced refs with zero incoming edges (W012); a
    file a field names is a value check's (@specforge/checks). Domain-specific
    validation is driven by declarative patterns from extensions.
    Note: detect_duplicate_entity_ids runs at all_files_parsed time
    (before graph construction). The remaining structural validators
    run after graph_built. Ordering enforced by validation_pipeline_ordering
    invariant (invariants/validation.spec).
  """
}

requirement FEATURE-017 "Diagnostic Reporting" {
  category "Feature"
  tier     v1
  text     """
    Validation diagnostics must be formatted with source context, fuzzy
    suggestions for typos, and aggregated into a final summary for
    human and machine consumption.

    Diagnostics are formatted with source snippets and underline markers
    pointing to the exact source location. Fuzzy matching suggests
    corrections for misspelled entity IDs. The aggregate summary collects
    all diagnostics with error/warning/info counts. All diagnostics are
    available in both human-readable (ariadne-rendered) and machine-parseable
    (structured JSON via --format=json) formats — agents consume the JSON
    format directly without parsing rendered output (per P3: agents are
    first-class consumers). Two of the six behaviors
    (format_diagnostics_with_source_context and provide_did_you_mean_suggestions)
    run inline during the resolution pass rather than as post-validation
    event consumers.
  """
}

requirement FEATURE-141 "Zero-Entity Validation" {
  category "Feature"
  tier     v1
  text     """
    Without extension-aware validation, entity keywords and field names
    from uninstalled or misconfigured extensions go undetected, producing
    silent graph corruption or misleading diagnostics.

    Group the four zero-entity validation behaviors under a dedicated
    feature: detect_unknown_entity_kinds checks keywords against the
    KindRegistry (E024), detect_unknown_entity_fields checks fields
    against the FieldRegistry (W020), two_phase_validate_semantic
    orchestrates the full Phase 2 semantic validation pass, and
    suggest_missing_extensions attaches help text to E024 diagnostics
    using the candidate vocabulary the registry
    build is handed (the builtins', generated with them, ADR 0122) that maps
    known keywords to their providing extensions. These behaviors execute in Phase 2 after
    registries are populated (see dynamic_entity_registration in
    features/zero-entity-core.spec).

    Bridge: detect_unknown_entity_kinds and detect_unknown_entity_fields
    reuse the same validation infrastructure as the zero-entity-core
    behaviors (behaviors/zero-entity-validation.spec). The KindRegistry
    and FieldRegistry are populated by extension registration behaviors
    (behaviors/zero-entity-registries.spec) and consumed by both
    validation paths.
  """
}
