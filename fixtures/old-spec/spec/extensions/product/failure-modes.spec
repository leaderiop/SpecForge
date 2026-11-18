// @specforge/product extension failure modes — FMEA risk analysis
//
// Failure modes specific to the product entity model:
// library dependency concerns.

use "extensions/product/invariants"

failure_mode module_cycle_detection_miss "Module Cycle Detection Miss" {
  invariant  module_dag
  severity   medium
  occurrence unlikely
  detection  moderate
  rpn        30
  cause      "Cycle detection in the module ModuleDependsOn graph misses indirect cycles through three or more modules"
  effect     "Topological sort of modules produces incorrect ordering or infinite loop during dependency resolution"
  mitigation "Use Tarjan's algorithm for the module dependency graph; fuzz test with randomly generated dependency graphs including transitive cycles"
  post_mitigation {
    severity   medium
    occurrence rare
    detection  certain
    rpn        5
  }
  verify unit "Module Cycle Detection Miss failure mode is handled"
}
