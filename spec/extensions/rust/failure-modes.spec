// @specforge/rust extension failure modes — FMEA risk analysis
//
// Failure modes specific to the Rust language integration:
// entity mapping accuracy and proc macro reliability.

use "extensions/rust/invariants"
failure_mode rust_entity_mapping_mismatch "Rust Entity Mapping Mismatch" {
  invariant  entity_mapping_precedence
  severity   6
  occurrence 3
  detection  5
  rpn        90

  cause      "Test function mapped to wrong entity due to naming collision in double-underscore convention — e.g., validate__input vs validate_input__"
  effect     "Coverage report attributes test results to wrong entity — misleading spec coverage metrics and false confidence"
  mitigation "Three-level precedence (tests field > proc macro > convention); ambiguous mappings produce diagnostics; strict mode rejects ambiguity"

  post_mitigation {
    severity   6
    occurrence 1
    detection  2
    rpn        12
  }
  verify unit "Rust Entity Mapping Mismatch failure mode is handled"
}

failure_mode rust_proc_macro_silent_drop "Rust Proc Macro Silent Drop" {
  invariant  entity_mapping_precedence
  severity   5
  occurrence 2
  detection  5
  rpn        50

  cause      "TestGuard Drop handler fails to record result — e.g., atexit handler not registered, file write fails, or process killed before Drop"
  effect     "Test result lost — entity shows as untested in coverage report despite passing test"
  mitigation "Atexit handler registered on first guard creation; fallback to JUnit XML collection; diagnostic when mapping file is missing expected entries"

  post_mitigation {
    severity   5
    occurrence 1
    detection  2
    rpn        10
  }
  verify unit "Rust Proc Macro Silent Drop failure mode is handled"
}
