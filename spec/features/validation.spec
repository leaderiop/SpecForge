// Validation feature

use behaviors/resolution
use behaviors/graph
use behaviors/validation
use behaviors/validation-ext
use behaviors/error-reporting
use behaviors/rust-collection

feature graph_validation "Graph Validation" {
  behaviors [
    resolve_use_imports, validate_import_cycles, link_entity_references, resolve_soft_cross_plugin_references, resolve_external_ref_declarations,
    build_in_memory_graph,
    detect_dangling_references, detect_duplicate_entity_ids, detect_orphan_behaviors,
    detect_unused_invariants, detect_unverified_behaviors, detect_orphan_events, validate_event_triggers,
    validate_persona_references, validate_surface_references,
    detect_orphan_refs, detect_orphan_features, detect_library_cycles, validate_behavior_ranges_in_roadmaps,
    validate_rpn_arithmetic, detect_unmitigated_high_risk_invariants, detect_orphan_capabilities, detect_features_with_empty_behaviors,
    detect_deliverables_with_no_capabilities, detect_orphan_libraries, detect_constraints_with_no_protected_invariants, detect_unused_glossary_terms,
    validate_empty_scenario, validate_duplicate_scenario_titles, validate_scenario_steps, validate_tests_field_references, validate_plugin_testability,
    validate_rust_entity_ids,
    format_diagnostics_with_source_context, provide_did_you_mean_suggestions, aggregate_diagnostic_summary,
  ]

  problem """
    .spec files contain cross-references between entities across multiple
    files. Broken references, duplicate IDs, import cycles, and orphan
    entities must be detected and reported with actionable diagnostics.
  """

  solution """
    Multi-pass validator that checks the in-memory graph for structural
    invariants: dangling references (E001), duplicate IDs (E002), import
    cycles (E003), orphans (W001-W011), and plugin-specific rules. All
    diagnostics are formatted in rustc style with suggestions.
  """
}
