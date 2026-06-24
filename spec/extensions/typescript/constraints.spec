// @specforge/typescript extension constraints
//
// Non-functional requirements specific to the TypeScript language integration:
// scan performance, mapping accuracy, Wasm binary size.

use "invariants/core"
use "extensions/typescript/invariants"
use "extensions/typescript/behaviors"
use "extensions/coverage/behaviors"

constraint ts_scan_performance "TypeScript Scan Performance" {
  description "Source scanning must complete within acceptable time for interactive use and watch mode."
  category    performance
  priority    critical

  metric """
    Full project scan: <5 seconds for 1000 files, <15 seconds for 5000 files.
    Incremental re-scan after single file edit: <200ms.
    Memory usage during scan: <256MB for 5000 files.
  """

  constrains [scan_typescript_project, extract_source_items, detect_react_components]
  protects [ts_export_completeness]

  verify unit "scan performance meets target for 1000-file project"
}

constraint ts_collection_accuracy "TypeScript Collection Accuracy" {
  description "Entity mapping has zero false positives and generated reports conform to the SpecforgeReport schema."
  category    reliability
  priority    critical

  metric """
    entity mapping has zero false positives; specforge-report.json
    conforms to the SpecforgeReport schema; all entity IDs are validated
    against the spec graph
  """

  constrains [collect_jest_results, collect_vitest_results, collect_playwright_results, collect_cypress_results, map_typescript_entity_ids, merge_monorepo_reports]
  protects [ts_entity_mapping_precedence]

  verify unit "entity mapping and report generation are accurate"
}

constraint ts_wasm_binary_size "TypeScript Extension Wasm Binary Size" {
  description "The Wasm binary must be small enough for fast download and AOT compilation."
  category    performance
  priority    high

  metric """
    Wasm binary size (gzipped): <2MB including tree-sitter-typescript grammar.
    AOT compilation time: <3 seconds on first use.
    Warm startup from cached AOT: <100ms.
  """

  constrains [scan_typescript_project, extract_source_items]

  verify unit "Wasm binary size within budget"
}
