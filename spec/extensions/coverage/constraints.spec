// @specforge/coverage extension constraints
//
// Non-functional requirements for coverage reporting and the
// Rust language integration: accuracy and report correctness.

use "extensions/coverage/behaviors"
use "extensions/coverage/invariants"
use "invariants/core"

constraint test_coverage_accuracy "Test Coverage Accuracy" {
  category   reliability
  priority   critical
  metric     """
    coverage percentage matches actual verified/total ratio;
    merge produces correct deduplicated results
  """
  constrains [
    merge_coverage_reports,
    compute_coverage_summary,
    gate_on_coverage_threshold,
    validate_test_ids_against_spec,
  ]
  protects   [testable_entity_classification, traceability_chain_integrity]

  verify unit "coverage percentage and merge are accurate"
}

constraint rust_collection_accuracy "Rust Collection Accuracy" {
  category   reliability
  priority   critical
  metric     """
    entity mapping has zero false positives; specforge-report.json
    conforms to the SpecforgeReport schema; all entity IDs are validated
  """
  constrains [
    rc_collect_rust_test_results,
    rc_parse_junit_xml,
    rc_parse_libtest_json,
    rc_resolve_entity_mapping,
    rc_validate_rust_entity_ids,
    rc_merge_workspace_reports,
    rc_emit_specforge_report_from_rust,
    rc_record_test_via_drop_guard,
  ]
  protects   [entity_mapping_precedence]

  verify unit "entity mapping and report generation are accurate"
}
