// @specforge/product extension failure modes — FMEA risk analysis
//
// Failure modes specific to the product entity model:
// library dependency concerns.

use extensions/product/invariants

failure_mode library_cycle_detection_miss "Library Cycle Detection Miss" {
  invariant  library_dag
  severity   5
  occurrence 2
  detection  3
  rpn        30

  cause      "Cycle detection in library depends_on graph misses indirect cycles through three or more libraries"
  effect     "Topological sort of libraries produces incorrect ordering or infinite loop during dependency resolution"
  mitigation "Use Tarjan's algorithm for library dependency graph; fuzz test with randomly generated dependency graphs including transitive cycles"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}
