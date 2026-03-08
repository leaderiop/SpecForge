// Core failure modes — FMEA risk analysis for the compiler engine
//
// Extension-specific failure modes live in their respective extension directories
// under spec/extensions/.

use invariants/core
use invariants/validation
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

  cause      "Bug in spec root detection allows two specforge.json files to coexist without error — e.g., one in the project root and one in a nested directory"
  effect     "Compiler uses unpredictable configuration — wrong extensions, wrong settings for all subsequent compilation"
  mitigation "Unit test: deliberate dual specforge.json files triggers error; project root detection checks for single config before resolution"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
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

failure_mode wasm_extension_crash "Wasm Extension Crash" {
  invariant  wasm_sandbox_integrity
  severity   6
  occurrence 3
  detection  3
  rpn        54

  cause      "Extension Wasm module traps during validate() or render() — e.g., out-of-bounds memory access, stack overflow, or unreachable instruction"
  effect     "Extension fails to complete its validation or export pass — diagnostics from that extension are lost, output may be incomplete"
  mitigation "Extism catches all traps and returns error; compiler wraps call in Result, emits ExtensionError with trap details; remaining extensions continue execution"

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

  cause      "specforge.http_get host function makes a request to an unresponsive service — extension blocks waiting for network response"
  effect     "Compilation hangs or takes excessively long — developer experiences unexplained delay"
  mitigation "Enforce timeout on all http_get calls (default 5s); fuel metering caps total execution time per extension; timeout produces diagnostic with URL"

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

  cause      "Extension A declares peer dependency on Extension B >=2.0, but Extension B version 1.x is installed — semver range check fails"
  effect     "Extension initialization fails — entity types from dependent extension are unavailable, soft references degrade silently"
  mitigation "Hard error on unsatisfied peer dependencies at startup; diagnostic includes installed vs required version; specforge add checks peers before installing"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}

failure_mode builtin_field_shadow "Grammar-Level Construct Shadow by Extension" {
  invariant  enhancement_builtin_precedence
  severity   8
  occurrence 2
  detection  2
  rpn        32

  cause      "Extension registers an enhancement field with the same name as a grammar-level construct (entity title, verify)"
  effect     "Grammar-level construct is shadowed — parser/resolver uses extension field definition instead of grammar-level syntax, causing unpredictable validation and broken contract extraction"
  mitigation "Enhancement registration checks every field name against the reserved grammar-level construct names; shadow attempt produces hard error E018 regardless of enhancement_policy; integration test with deliberate shadow attempt"

  post_mitigation {
    severity   8
    occurrence 1
    detection  1
    rpn        8
  }
}

failure_mode aot_cache_corruption "AOT Cache Corruption" {
  invariant  aot_cache_integrity
  severity   5
  occurrence 2
  detection  4
  rpn        40

  cause      "AOT compiled artifact in .specforge/cache/ is corrupted — e.g., interrupted write, disk error, or platform mismatch after OS upgrade"
  effect     "Extension fails to load from cache — confusing error message if corruption not detected; potential wrong behavior if partially loaded"
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

  cause      "Extension A declares peer dependency on Extension B, which declares peer dependency on Extension A — circular chain prevents topological sort"
  effect     "Topological sort fails, all extension functionality blocked — no extension entities, no extension validation, no extension generation"
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

  cause      "Extension built against an outdated manifest schema — field names and semantics differ between versions"
  effect     "Manifest fields misinterpreted — entity registrations wrong, peer dependencies ignored, sandbox policy defaults applied instead of declared values"
  mitigation "manifestVersion is validated at load time; unsupported versions produce a hard error with upgrade instructions; unknown fields produce warnings"

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

  cause      "Extension sends malformed or unexpected data through a host function — e.g., invalid JSON to specforge.add_graph_node, wrong schema to specforge.emit_diagnostic"
  effect     "Host processes corrupted data — wrong graph nodes added, invalid diagnostics emitted, graph corruption possible"
  mitigation "Schema validation on every host function input; malformed data returns ExtensionError to extension; integration tests with deliberately malformed extension inputs"

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

  cause      "Two extensions register the same entity kind name but the KindRegistry fails to detect the collision — e.g., race condition or case-insensitive match not checked"
  effect     "One extension's entity kind silently shadows the other — entities parsed incorrectly, wrong validation rules applied, corrupted graph"
  mitigation "KindRegistry checks all registrations against reserved words and existing extension kinds; duplicate registration returns hard error E022/E023; property-based tests with random kind name combinations"

  post_mitigation {
    severity   7
    occurrence 1
    detection  1
    rpn        7
  }
}

failure_mode registry_unavailability "Registry Unavailability" {
  invariant  registry_integrity
  severity   4
  occurrence 3
  detection  2
  rpn        24

  cause      "Registry endpoint is unreachable — DNS failure, network timeout, authentication error, or registry service outage"
  effect     "Extension installation or upgrade fails — developer cannot add new extensions or update existing ones"
  mitigation "Configurable timeout (default 10s) with retry guidance in diagnostic; offline fallback to local cache; diagnostic includes registry URL and HTTP status"

  post_mitigation {
    severity   4
    occurrence 2
    detection  1
    rpn        8
  }
}

failure_mode collector_output_malformation "Collector Output Malformation" {
  invariant  collector_output_conformance
  severity   5
  occurrence 2
  detection  3
  rpn        30

  cause      "Collector extension produces output that does not conform to specforge-report/v1 schema — e.g., missing entries array, invalid entity IDs, wrong schema version"
  effect     "Coverage ingestion fails or produces incorrect results — developer sees wrong coverage statistics or missing test mappings"
  mitigation "Schema validation on all collector output before ingestion; malformed output produces ExtensionError with specific field-level details; partial ingestion of valid entries with warnings for invalid ones"

  post_mitigation {
    severity   5
    occurrence 1
    detection  1
    rpn        5
  }
}

failure_mode extension_initialization_failure "Extension Initialization Failure" {
  invariant  extension_isolation
  severity   6
  occurrence 3
  detection  2
  rpn        36

  cause      "Extension .wasm module missing or exporting wrong initialize() signature — e.g., built with incompatible PDK version"
  effect     "Extension entities not registered — references to extension entities produce E001 instead of I004, misleading developers into thinking entities are misspelled"
  mitigation "Detect missing/wrong initialize() at loadModule phase before any export calls; transition extension to failed state; emit diagnostic with PDK version hint; continue loading remaining extensions"

  post_mitigation {
    severity   6
    occurrence 1
    detection  1
    rpn        6
  }
}

failure_mode grammar_conflict_between_extensions {
  severity 6
  occurrence 4
  detection 3
  rpn 72

  cause "Two extensions declare grammars for the same entity kind with no conflict resolution policy configured."
  effect "Ambiguous grammar composition leads to unpredictable parsing results or compilation failure."
  mitigation "GrammarConflictPolicy (error | priority | namespace) is required when multiple extensions target the same entity kind. Default policy is error (fail fast)."

  invariant grammar_composition_determinism

  post_mitigation {
    severity 3
    occurrence 2
    detection 2
    rpn 12
  }
}

failure_mode body_parser_crash {
  severity 7
  occurrence 3
  detection 2
  rpn 42

  cause "Extension body parser Wasm export panics, exceeds timeout, or returns malformed JSON."
  effect "Entity body cannot be parsed; compilation for affected entities fails."
  mitigation "Wasm sandbox isolates crashes. Timeout enforcement (configurable, default 5000ms). Fallback to raw string field on parser error with diagnostic warning. Output JSON validated against declared schema before acceptance."

  invariant body_parser_output_conformance

  post_mitigation {
    severity 4
    occurrence 2
    detection 1
    rpn 8
  }
}

failure_mode grammar_version_mismatch {
  severity 5
  occurrence 5
  detection 4
  rpn 100

  cause "Extension provides grammar .wasm compiled for a different tree-sitter ABI version than the host runtime."
  effect "Grammar loading fails or produces incorrect parse trees silently."
  mitigation "ABI version validation during grammar loading. Compiler reports GrammarError with expected vs actual ABI version. Grammar cache invalidation on ABI version change."

  invariant grammar_injection_isolation

  post_mitigation {
    severity 2
    occurrence 2
    detection 1
    rpn 4
  }
}
