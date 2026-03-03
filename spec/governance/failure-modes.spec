// Failure Modes — FMEA risk analysis

use invariants/core
use invariants/validation
use invariants/rust
use invariants/formatting

failure_mode incremental_divergence "Incremental Divergence" {
  invariant  incremental_correctness
  severity   7
  occurrence 3
  detection  4
  rpn        84

  cause      "Bug in invalidation logic misses a transitive dependent, leaving stale nodes in graph"
  effect     "Incremental build produces different diagnostics than cold rebuild — user sees phantom errors or missed errors"
  mitigation "Property test comparing incremental result to cold rebuild for randomized file changes"

  post_mitigation {
    severity   7
    occurrence 1
    detection  2
    rpn        14
  }
}

failure_mode string_interning_collision "String Interning Collision" {
  invariant  string_interning_consistency
  severity   8
  occurrence 1
  detection  6
  rpn        48

  cause      "Hash collision in the interning table causes two different strings to share the same key"
  effect     "Two distinct entity IDs compare as equal — phantom duplicate ID errors or missed reference errors"
  mitigation "Use a collision-resistant hash (lasso uses fx-hash); add debug-mode assertion comparing string values on every key lookup"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
}

failure_mode duplicate_id_detection_miss "Duplicate ID Detection Miss" {
  invariant  entity_id_uniqueness
  severity   7
  occurrence 2
  detection  3
  rpn        42

  cause      "Race condition or ordering bug in parallel file processing skips duplicate detection for entities declared in different files"
  effect     "Two entities with the same ID exist in the graph — unpredictable behavior during validation and rendering"
  mitigation "Serial ID registration with a global lock; integration tests with deliberately duplicated IDs across files"

  post_mitigation {
    severity   7
    occurrence 1
    detection  1
    rpn        7
  }
}

failure_mode import_cycle_detection_miss "Import Cycle Detection Miss" {
  invariant  import_dag
  severity   5
  occurrence 2
  detection  3
  rpn        30

  cause      "Topological sort algorithm has a bug that misses cycles in graphs with specific structures (e.g., self-referential imports)"
  effect     "Import cycle goes undetected — infinite loop during resolution or stack overflow"
  mitigation "Use Tarjan's algorithm with proven correctness; fuzz test with randomly generated import graphs including self-cycles"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

failure_mode diagnostic_drop_under_error_collection "Diagnostic Drop Under Error Collection" {
  invariant  multi_error_collection
  severity   6
  occurrence 2
  detection  5
  rpn        60

  cause      "Error in diagnostic collection logic silently drops diagnostics when the bag exceeds an internal limit or encounters an unexpected error type"
  effect     "User misses errors — believes spec is clean when it is not, leading to downstream failures"
  mitigation "Diagnostic bag has no size limit; every code path that produces a diagnostic uses the same collector; integration test asserting diagnostic count matches expected for a known-bad spec"

  post_mitigation {
    severity   6
    occurrence 1
    detection  2
    rpn        12
  }
}

failure_mode silent_reference_swallow "Silent Reference Swallow" {
  invariant  reference_resolution_completeness
  severity   8
  occurrence 2
  detection  4
  rpn        64

  cause      "Bug in reference resolution silently skips a reference instead of emitting E001 or I004 — e.g., an early return in a match arm"
  effect     "Broken reference goes undetected — user believes spec is clean when a dangling reference exists, leading to incorrect traceability"
  mitigation "Exhaustive integration test with deliberately broken references for every edge type; fuzzing with random ID mutations"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
}

failure_mode spec_root_duplication "Spec Root Duplication" {
  invariant  spec_root_singleton
  severity   5
  occurrence 2
  detection  2
  rpn        20

  cause      "Bug in spec root detection allows two spec blocks to coexist without error — e.g., one in specforge.spec and one in a nested file"
  effect     "Compiler uses unpredictable configuration — wrong infix, wrong plugins, wrong settings for all subsequent compilation"
  mitigation "Unit test: deliberate dual spec blocks across files triggers error; parsing stage checks global count before resolution"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

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

failure_mode traceability_gap_undetected "Traceability Gap Undetected" {
  invariant  traceability_chain_integrity
  severity   7
  occurrence 3
  detection  4
  rpn        84

  cause      "A testable entity lacks test linkage or test execution proof but specforge trace fails to flag it — e.g., a missing branch in the coverage level computation"
  effect     "Team believes spec is fully covered when gaps exist — untested behaviors ship to production without detection"
  mitigation "Four-level coverage model (declared/linked/executed/passing) catches gaps at each layer; integration tests with deliberately incomplete traceability chains"

  post_mitigation {
    severity   7
    occurrence 1
    detection  2
    rpn        14
  }
}

failure_mode non_deterministic_diagnostic_order "Non-Deterministic Diagnostic Order" {
  invariant  diagnostic_determinism
  severity   4
  occurrence 3
  detection  4
  rpn        48

  cause      "HashMap iteration order or parallel file processing produces different diagnostic ordering across runs"
  effect     "CI produces flaky results — same spec files yield different diagnostic output, confusing developers and breaking snapshot tests"
  mitigation "Sort diagnostics by (file_path, line, column, code) before emission; property test asserting identical output across 100 runs"

  post_mitigation {
    severity   4
    occurrence 1
    detection  1
    rpn        4
  }
}

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
}

failure_mode rust_drift_false_negative "Rust Drift False Negative" {
  invariant  rust_drift_detection_accuracy
  severity   7
  occurrence 2
  detection  4
  rpn        56

  cause      "Checksum header parsing bug or SHA256 computation error causes stale generated Rust code to pass drift detection"
  effect     "CI passes with out-of-date generated code — production code diverges from spec without detection"
  mitigation "Property test: mutate spec then verify --check detects drift; integration test with known stale files"

  post_mitigation {
    severity   7
    occurrence 1
    detection  2
    rpn        14
  }
}

failure_mode non_deterministic_rust_output "Non-Deterministic Rust Output" {
  invariant  deterministic_rust_generation
  severity   6
  occurrence 2
  detection  3
  rpn        36

  cause      "HashMap iteration order, filesystem ordering, or timestamps leak into generated Rust code"
  effect     "specforge gen rust --check falsely reports drift; CI flakes; generated code diffs on unchanged specs"
  mitigation "Sort all collections before emission; property test: generate twice and compare SHA256; ban timestamp/random in codegen paths"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}

failure_mode formatting_idempotency_violation "Formatting Idempotency Violation" {
  invariant  formatting_idempotency
  severity   7
  occurrence 3
  detection  3
  rpn        63

  cause      "Bug in alignment or wrapping rules causes the formatter to oscillate between two states — e.g., a reference list that alternates between inline and multi-line on successive runs"
  effect     "specforge format --check flakes in CI — developers cannot achieve clean formatting, losing trust in the tool"
  mitigation "Property-based tests with random valid .spec files verify format(format(x)) == format(x); regression test for every reported violation; alignment rules use stable column computation"

  post_mitigation {
    severity   7
    occurrence 1
    detection  1
    rpn        7
  }
}

failure_mode comment_loss_during_formatting "Comment Loss During Formatting" {
  invariant  comment_preservation
  severity   8
  occurrence 2
  detection  4
  rpn        64

  cause      "Comment attachment algorithm fails on edge cases — e.g., comment between closing brace and next block, or comment inside an empty block body"
  effect     "User loses documentation comments after running the formatter — silent data loss that may go unnoticed until much later"
  mitigation "Comment count assertion: formatted output must contain the same number of comment tokens as input; fuzzing with comment-heavy .spec files; diff review mode shows comment changes"

  post_mitigation {
    severity   8
    occurrence 1
    detection  2
    rpn        16
  }
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
}
