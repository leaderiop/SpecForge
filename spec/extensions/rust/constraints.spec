// @specforge/rust extension constraints
//
// Non-functional requirements specific to the Rust language integration:
// entity mapping accuracy and report correctness.

use "invariants/core"
use "extensions/rust/invariants"
use "extensions/rust/behaviors"
use "extensions/coverage/behaviors"
constraint test_coverage_accuracy "Test Coverage Accuracy" {
  category    reliability
  priority    critical

  metric """
    coverage percentage matches actual verified/total ratio;
    merge produces correct deduplicated results
  """

  constrains [merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold, validate_test_ids_against_spec]
  protects [testable_entity_classification, traceability_chain_integrity]

  verify unit "coverage percentage and merge are accurate"

}

constraint rust_collection_accuracy "Rust Collection Accuracy" {
  category    reliability
  priority    critical

  metric """
    entity mapping has zero false positives; specforge-report.json
    conforms to the SpecforgeReport schema; all entity IDs are validated
  """

  constrains [collect_rust_test_results, parse_junit_xml, parse_libtest_json, resolve_entity_mapping, validate_rust_entity_ids, merge_workspace_reports, emit_specforge_report_from_rust, record_test_via_drop_guard]
  protects [entity_mapping_precedence]

  verify unit "entity mapping and report generation are accurate"

}
