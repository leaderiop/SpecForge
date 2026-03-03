// Failure Modes — FMEA risk analysis

use invariants/core
use invariants/validation
use invariants/rust
use invariants/formatting
use invariants/wasm

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

  cause      "Bug in spec root detection allows two spec blocks to coexist without error — e.g., one in specforge.json and one in a nested file"
  effect     "Compiler uses unpredictable configuration — wrong plugins, wrong settings for all subsequent compilation"
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

failure_mode wasm_plugin_crash "Wasm Plugin Crash" {
  invariant  wasm_sandbox_integrity
  severity   6
  occurrence 3
  detection  3
  rpn        54

  cause      "Plugin Wasm module traps during validate() or generate() — e.g., out-of-bounds memory access, stack overflow, or unreachable instruction"
  effect     "Plugin fails to complete its validation or generation pass — diagnostics from that plugin are lost, output may be incomplete"
  mitigation "Extism catches all traps and returns error; compiler wraps call in Result, emits PackageError with trap details; remaining plugins continue execution"

  post_mitigation {
    severity   6
    occurrence 1
    detection  2
    rpn        12
  }
}

failure_mode wasm_host_function_timeout "Wasm Host Function Timeout" {
  invariant  wasm_sandbox_integrity
  severity   5
  occurrence 3
  detection  2
  rpn        30

  cause      "specforge.http_get host function makes a request to an unresponsive service — plugin blocks waiting for network response"
  effect     "Compilation hangs or takes excessively long — developer experiences unexplained delay"
  mitigation "Enforce timeout on all http_get calls (default 10s); fuel metering caps total execution time per plugin; timeout produces diagnostic with URL"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

failure_mode peer_dependency_version_mismatch "Peer Dependency Version Mismatch" {
  invariant  peer_dependency_satisfaction
  severity   6
  occurrence 2
  detection  2
  rpn        24

  cause      "Plugin A declares peer dependency on Plugin B >=2.0, but Plugin B version 1.x is installed — semver range check fails"
  effect     "Plugin initialization fails — entity types from dependent plugin are unavailable, soft references degrade silently"
  mitigation "Hard error on unsatisfied peer dependencies at startup; diagnostic includes installed vs required version; specforge add checks peers before installing"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}

failure_mode builtin_field_shadow "Built-in Field Shadow by Plugin" {
  invariant  enhancement_builtin_precedence
  severity   8
  occurrence 2
  detection  2
  rpn        32

  cause      "Plugin registers an enhancement field with the same name as a built-in field"
  effect     "Built-in field is shadowed — parser/resolver uses plugin field definition instead of built-in, causing unpredictable validation and broken contract extraction"
  mitigation "Enhancement registration checks every field name against the built-in FieldRegistry; shadow attempt produces hard error E018 regardless of enhancement_policy; integration test with deliberate shadow attempt"

  post_mitigation {
    severity   8
    occurrence 1
    detection  1
    rpn        8
  }
}

failure_mode aot_cache_corruption "AOT Cache Corruption" {
  invariant  plugin_load_order_determinism
  severity   5
  occurrence 2
  detection  4
  rpn        40

  cause      "AOT compiled artifact in .specforge/cache/ is corrupted — e.g., interrupted write, disk error, or platform mismatch after OS upgrade"
  effect     "Plugin fails to load from cache — confusing error message if corruption not detected; potential wrong behavior if partially loaded"
  mitigation "Content-hash verification on cache load; corrupted entries evicted and recompiled; platform string in cache filename prevents cross-platform misuse"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

failure_mode circular_peer_dependency "Circular Peer Dependency" {
  invariant  peer_dependency_satisfaction
  severity   6
  occurrence 2
  detection  2
  rpn        24

  cause      "Plugin A declares peer dependency on Plugin B, which declares peer dependency on Plugin A — circular chain prevents topological sort"
  effect     "Topological sort fails, all plugin functionality blocked — no plugin entities, no plugin validation, no plugin generation"
  mitigation "Tarjan's cycle detection during topological sort; full cycle path included in diagnostic message; specforge doctor reports cycle with resolution suggestions"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}

failure_mode manifest_schema_mismatch "Manifest Schema Mismatch" {
  invariant  peer_dependency_satisfaction
  severity   5
  occurrence 2
  detection  3
  rpn        30

  cause      "Plugin built against manifest v1 schema loaded by a v2 runtime — field names and semantics differ between versions"
  effect     "Manifest fields misinterpreted — entity registrations wrong, peer dependencies ignored, sandbox policy defaults applied instead of declared values"
  mitigation "manifestVersion is the first field checked; v1 on v2 runtime produces migration error with upgrade instructions; unknown fields produce warnings"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

failure_mode host_function_type_violation "Host Function Type Safety Violation" {
  invariant  host_function_type_safety
  severity   8
  occurrence 2
  detection  3
  rpn        48

  cause      "Plugin sends malformed or unexpected data through a host function — e.g., invalid JSON to specforge.register_entity, wrong schema to specforge.emit_diagnostic"
  effect     "Host processes corrupted data — wrong entities registered, invalid diagnostics emitted, graph corruption possible"
  mitigation "Schema validation on every host function input; malformed data returns PackageError to plugin; integration tests with deliberately malformed plugin inputs"

  post_mitigation {
    severity   8
    occurrence 1
    detection  1
    rpn        8
  }
}

failure_mode entity_kind_collision_undetected "Entity Kind Collision Undetected" {
  invariant  entity_kind_uniqueness
  severity   7
  occurrence 2
  detection  2
  rpn        28

  cause      "Two plugins register the same entity kind name but the KindRegistry fails to detect the collision — e.g., race condition or case-insensitive match not checked"
  effect     "One plugin's entity kind silently shadows the other — entities parsed incorrectly, wrong validation rules applied, corrupted graph"
  mitigation "KindRegistry checks all registrations against built-ins and existing plugin kinds; duplicate registration returns hard error E022/E023; property-based tests with random kind name combinations"

  post_mitigation {
    severity   7
    occurrence 1
    detection  1
    rpn        7
  }
}

failure_mode plugin_initialization_failure "Plugin Initialization Failure" {
  invariant  plugin_isolation
  severity   6
  occurrence 3
  detection  2
  rpn        36

  cause      "Plugin .wasm module missing or exporting wrong initialize() signature — e.g., built with incompatible PDK version"
  effect     "Plugin entities not registered — references to plugin entities produce E001 instead of I004, misleading developers into thinking entities are misspelled"
  mitigation "Detect missing/wrong initialize() at loadModule phase before any export calls; transition plugin to failed state; emit diagnostic with PDK version hint; continue loading remaining plugins"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}
