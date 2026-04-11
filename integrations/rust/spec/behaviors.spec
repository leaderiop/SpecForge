// specforge-test crate behaviors
// Organized by phase: Phase 1 (annotation + collection),
// Phase 2 (build.rs integration), Phase 3 (coverage summary).

use "types"
use "invariants"
use "ports"
use "events"

// ============================================================
// Phase 1: Annotation + Collection (Option A)
// ============================================================

behavior expand_test_attribute "Expand Test Attribute" {
  category command
  invariants [zero_compiler_dependency, should_panic_incompatibility]
  types      [TestGuard]

  contract """
    The #[specforge::test(...)] proc macro MUST expand to inject a TestGuard
    at the start of the annotated function body. It MUST NOT replace or
    interfere with #[test], #[tokio::test], #[rstest], or other test
    attributes. It MUST parse entity_kind and entity_id from the attribute
    arguments. Multiple #[specforge::test(...)] attributes on the same
    function MUST each produce a separate TestGuard.
  """

  requires {
    valid_attribute_args "attribute contains entity_kind (any identifier) and entity_id"
  }

  ensures {
    guard_injected    "function body begins with a TestGuard::new() binding"
    test_attr_intact  "original #[test] or framework attribute is preserved"
    multi_attr_works  "N attributes produce N guards"
  }

  verify unit "single behavior attribute expands correctly"
  verify unit "verify description attribute includes slug in guard"
  verify unit "multiple attributes on same function produce multiple guards"
  verify unit "attribute alongside #[tokio::test] preserves async"
  verify unit "attribute alongside #[rstest] preserves parameterization"
  verify unit "should_panic combined with specforge::test emits compile warning"
}

behavior record_test_result_on_drop "Record Test Result on Drop" {
  category command
  invariants [drop_guard_correctness]
  types      [TestGuard, TestRecordEntry, TestOutcome]
  ports      [TestRegistry]
  produces   [test_result_recorded]

  contract """
    When a TestGuard is dropped, it MUST check std::thread::panicking()
    to determine the test outcome. It MUST push a TestRecordEntry to the
    thread-safe TestRegistry. Pass if not panicking, fail if panicking.
    The guard MUST NOT catch or suppress panics.
  """

  requires {
    guard_is_live "TestGuard was constructed and not yet dropped"
  }

  ensures {
    entry_recorded   "TestRegistry contains exactly one new entry for this guard"
    status_correct   "status is fail iff std::thread::panicking() was true"
    no_panic_change  "guard does not alter panic propagation"
  }

  verify unit "passing test records pass"
  verify unit "panicking test records fail"
  verify unit "guard does not catch the panic"
  verify property "concurrent drops do not lose entries"
}

behavior emit_binary_report "Emit Binary Report" {
  category command
  invariants [atexit_write_once, zero_compiler_dependency]
  types      [BinaryReport, TestRecordEntry]
  ports      [ProcessExit, FileSystem, TestRegistry]
  consumes   [test_result_recorded]
  produces   [binary_report_written]

  contract """
    At process exit, the atexit handler MUST drain all entries from the
    TestRegistry, construct a BinaryReport, and write it to
    target/specforge/<binary-name>.json. It MUST use std::sync::Once to
    guarantee single invocation. If the registry is empty (no specforge
    tests ran), it MUST NOT write a report file.
  """

  requires {
    atexit_registered  "atexit handler was registered during first TestGuard creation"
  }

  ensures {
    report_written     "target/specforge/<binary>.json exists with all collected entries"
    empty_skipped      "no file written when zero entries collected"
    valid_json         "report file is valid JSON matching BinaryReport schema"
  }

  verify unit "report contains all recorded test entries"
  verify unit "empty registry produces no file"
  verify unit "report file is valid JSON"
  verify unit "binary name is correctly resolved"
  verify integration "multi-threaded test suite produces complete report"
}

behavior resolve_convention_mapping "Resolve Convention Mapping" {
  category query
  invariants [convention_separator_unambiguous]
  types      [EntityMappingEntry, MappingResolutionLevel]

  contract """
    Given a test function name, the resolver MUST attempt to extract an
    entity ID using the double-underscore convention: {entity_id}__{slug}.
    If no double underscore exists, it MUST check if the enclosing module
    name matches a known entity ID. The slugification algorithm MUST be
    deterministic: lowercase, spaces to underscores, angle brackets to
    lt/gt/lte/gte, strip non-alphanumeric, collapse consecutive underscores.
  """

  requires {
    test_name_non_empty "test function name is a non-empty string"
  }

  ensures {
    mapping_resolved   "returns Some(EntityMappingEntry) if convention matches, None otherwise"
    slug_deterministic "same verify description always produces same slug"
  }

  verify unit "double underscore splits entity_id from slug"
  verify unit "module name fallback matches entity ID"
  verify unit "no double underscore and no module match returns None"
  verify unit "slugification handles spaces"
  verify unit "slugification handles angle brackets"
  verify unit "slugification handles special characters"
  verify unit "slugification collapses consecutive underscores"
  verify property "slugify is idempotent"
}

behavior register_atexit_handler "Register Atexit Handler" {
  category command
  invariants [atexit_write_once]
  ports      [ProcessExit]

  contract """
    The first TestGuard construction in a process MUST register the atexit
    handler via std::sync::Once. Subsequent TestGuard constructions MUST
    NOT re-register. The handler MUST be registered using libc::atexit
    on POSIX systems.
  """

  requires {
    first_guard "at least one TestGuard is being constructed"
  }

  ensures {
    registered_once "atexit handler is registered exactly once"
  }

  verify unit "first guard triggers atexit registration"
  verify unit "second guard does not re-register"
}

// ============================================================
// Phase 2: build.rs Integration
// ============================================================

behavior invoke_specforge_export "Invoke Specforge Export" {
  category command
  invariants [graceful_degradation]
  types      [GraphExport]
  ports      [SpecforgeCli, FileSystem]
  produces   [graph_export_refreshed]

  contract """
    The build.rs script MUST invoke `specforge export --format=graph
    --output target/specforge/graph.json`. If the specforge binary is
    not found or the command fails, build.rs MUST emit a cargo::warning
    and continue without error. It MUST set cargo::rerun-if-changed=spec/
    so the export refreshes when spec files change.
  """

  requires {
    build_script_context "running inside a Cargo build.rs execution context"
  }

  ensures {
    graph_written_or_skipped "target/specforge/graph.json exists, or a cargo warning was emitted"
    rerun_directive_set      "cargo::rerun-if-changed=spec/ is printed to stdout"
  }

  verify unit "specforge on PATH produces graph.json"
  verify unit "specforge not on PATH emits warning and succeeds"
  verify unit "specforge command failure emits warning and succeeds"
  verify unit "rerun-if-changed is always emitted"
  verify integration "graph.json matches GraphExport schema"
}

behavior generate_entity_constants "Generate Entity Constants" {
  category command
  invariants [graceful_degradation]
  types      [GraphExport, ExportedEntity]
  ports      [FileSystem]
  consumes   [graph_export_refreshed]

  contract """
    When graph.json is available, build.rs MAY generate a Rust module
    containing const &str values for each entity ID. This module MUST
    be written to OUT_DIR and included via include!(concat!(env!("OUT_DIR"),
    "/specforge_entities.rs")). If graph.json is not available, the
    module MUST NOT be generated and the include! MUST be behind a
    cfg attribute.
  """

  requires {
    graph_export_present "target/specforge/graph.json exists and is valid"
  }

  ensures {
    constants_match_graph "every entity in graph.json has a corresponding const"
    names_are_valid_rust  "entity IDs are converted to valid SCREAMING_SNAKE_CASE identifiers"
  }

  verify unit "entity IDs are converted to SCREAMING_SNAKE_CASE"
  verify unit "hyphens in IDs are converted to underscores"
  verify unit "generated module compiles"
  verify unit "missing graph.json skips generation"
}

// ============================================================
// Phase 3: Coverage Summary at Test Exit (Option C)
// ============================================================

behavior load_graph_at_exit "Load Graph at Exit" {
  category query
  invariants [graceful_degradation]
  types      [GraphExport, ExportedEntity]
  ports      [FileSystem]

  contract """
    During the atexit handler, after draining the test registry, the
    handler MUST attempt to read target/specforge/graph.json. If the
    file does not exist or cannot be parsed, the handler MUST skip the
    coverage summary silently. The graph MUST NOT be loaded during test
    execution — only at exit.
  """

  requires {
    atexit_context "executing inside the atexit handler after tests complete"
  }

  ensures {
    graph_loaded_or_skipped "GraphExport is available, or coverage summary is skipped"
    no_load_during_tests    "graph is not read until atexit fires"
  }

  verify unit "valid graph.json is parsed into GraphExport"
  verify unit "missing graph.json causes silent skip"
  verify unit "malformed graph.json causes silent skip"
}

behavior compute_coverage_diff "Compute Coverage Diff" {
  category query
  types      [GraphExport, TestRecordEntry, CoverageDiff, CoverageDiffStatus]

  contract """
    Given a GraphExport and collected TestRecordEntries, the system MUST
    compute a CoverageDiff per testable entity: expected verify count from
    the graph, covered count from matching test records, and a status
    (fully_covered, partially_covered, uncovered, no_intent). Entities
    without verify statements MUST be reported as no_intent.
  """

  requires {
    graph_available   "GraphExport is loaded"
    entries_drained   "TestRecordEntries are collected from registry"
  }

  ensures {
    all_testable_covered "every testable entity in graph appears in the diff"
    counts_accurate      "expected = graph verify count, covered = matching test count"
    status_derived       "status is fully_covered iff covered >= expected and expected > 0"
  }

  verify unit "entity with 3 verify and 3 tests is fully_covered"
  verify unit "entity with 3 verify and 1 test is partially_covered"
  verify unit "entity with 2 verify and 0 tests is uncovered"
  verify unit "entity with 0 verify is no_intent"
  verify unit "non-testable entities are excluded"
  verify unit "all verify covered but some failing is covered_with_failures"
  verify unit "partial coverage with mixed pass/fail counted correctly"
}

behavior print_coverage_summary "Print Coverage Summary" {
  category command
  invariants [graceful_degradation]
  types      [CoverageDiff, CoverageDiffStatus, GraphExport]
  ports      [Stderr, FileSystem]
  consumes   [binary_report_written]

  contract """
    After computing the coverage diff, the atexit handler MUST print a
    compact summary to stderr showing each testable entity with its
    coverage status. The summary MUST include the graph export timestamp
    so staleness is visible. If no graph export is available, the summary
    MUST be skipped silently. The summary MUST be printed only when at
    least one #[specforge::test] was collected.
  """

  requires {
    diff_computed    "CoverageDiff list is available"
    stderr_writable  "stderr is open for writing"
  }

  ensures {
    summary_printed      "stderr shows coverage table with entity/expected/covered/status"
    timestamp_visible    "graph export timestamp is included in header"
    skipped_when_no_graph "no output when graph.json was unavailable"
    skipped_when_no_tests "no output when zero specforge tests ran"
  }

  verify unit "summary includes all testable entities"
  verify unit "timestamp from graph export appears in header"
  verify unit "no graph means no output"
  verify unit "no specforge tests means no output"
  verify integration "summary format matches expected layout"
}
