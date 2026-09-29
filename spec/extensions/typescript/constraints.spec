// @specforge/typescript extension constraints
//
// Non-functional requirements specific to the TypeScript language integration:
// scan performance, mapping accuracy, Wasm binary size.

use "extensions/typescript/behaviors"
use "extensions/typescript/invariants"
use "invariants/core"

constraint ts_scan_performance "TypeScript Scan Performance" {
  description "Source scanning must complete within acceptable time for interactive use and watch mode."
  category    performance
  priority    critical
  metric      """
    Full project scan: <5 seconds for 1000 files, <15 seconds for 5000 files.
    Incremental re-scan after single file edit: <200ms.
    Memory usage during scan: <256MB for 5000 files.
  """
  constrains  [scan_typescript_project, extract_source_items, detect_react_components]
  protects    [ts_export_completeness]
  verify unit "scan performance meets target for 1000-file project"
}

constraint ts_collection_accuracy "TypeScript Mapping Accuracy" {
  description "Source-to-entity mapping has zero false positives."
  category    reliability
  priority    critical
  metric      """
    source-to-entity mapping has zero false positives; every mapped ID is
    validated against the spec graph
  """
  constrains  [map_typescript_entity_ids]
  protects    [ts_entity_mapping_precedence]
  verify unit "entity mapping and report generation are accurate"
}

constraint ts_wasm_binary_size "TypeScript Extension Wasm Binary Size" {
  description "The Wasm binary must be small enough for fast download and first compilation."
  category    performance
  priority    high
  metric      """
    Wasm binary size (gzipped): <2MB including tree-sitter-typescript grammar.
    First compilation: <3 seconds on a cold compile cache.
    Compile-cache-hit startup: <100ms.
  """
  constrains  [scan_typescript_project, extract_source_items]
  verify unit "Wasm binary size within budget"
}
