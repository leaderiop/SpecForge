// @specforge/rust extension behaviors — Rust test collection

use invariants/core
use extensions/rust/invariants
use types/core
use types/diagnostics
use extensions/coverage/types
use extensions/rust/types
use ports/outbound
use extensions/rust/events
use extensions/rust/decisions

behavior collect_rust_test_results "Collect Rust Test Results" {
  types      [SpecforgeReport, TestResultEntry, CollectFormat]
  ports      [TestReporter, FileSystem]
  invariants [entity_mapping_precedence]
  produces   [rust_tests_collected]

  contract """
    When specforge collect rust is invoked, the system MUST parse test
    output from the specified format (stdin, JUnit XML, or JSON), match
    test names to entity IDs using three-level precedence, and emit
    specforge-report.json.
  """

  verify unit "cargo test text output is parsed"
  verify unit "nextest JUnit XML is parsed"
  verify unit "libtest JSON is parsed"
  verify integration "end-to-end collect from cargo test to specforge-report.json"

}

behavior parse_junit_xml "Parse JUnit XML" {
  types      [TestResultEntry, CollectFormat]
  ports      [RustTestOutputParser]
  adrs       [nextest_junit_primary_format]

  contract """
    The system MUST parse JUnit XML produced by cargo-nextest. Each
    <testcase> element MUST be mapped to a TestResultEntry. The classname
    attribute MUST be used for module path extraction. Failures MUST
    capture the <failure> message.
  """

  verify unit "testcase elements map to TestResultEntry"
  verify unit "classname extracts module path"
  verify unit "failure message is captured"

}

behavior parse_libtest_json "Parse Libtest JSON" {
  types      [TestResultEntry, CollectFormat]
  ports      [RustTestOutputParser]

  contract """
    The system MUST parse NDJSON output from cargo test --format json
    (unstable). Each test event MUST be mapped to a TestResultEntry.
    The system MUST handle both libtest and nextest JSON formats.
  """

  verify unit "libtest JSON events map to TestResultEntry"
  verify unit "nextest JSON events map to TestResultEntry"

}

behavior resolve_entity_mapping "Resolve Entity Mapping" {
  types      [EntityMappingEntry, MappingResolutionLevel]
  invariants [entity_mapping_precedence]

  contract """
    The system MUST resolve test-to-entity mappings using three-level
    precedence: (1) tests field in .spec files (authoritative), (2)
    #[specforge::test] proc macro attribute (explicit), (3) module
    name / double-underscore convention (implicit). Higher levels
    MUST override lower levels. Ambiguous mappings MUST be reported.
  """

  verify unit "tests field mapping takes highest precedence"
  verify unit "proc macro attribute overrides naming convention"
  verify unit "double-underscore convention extracts entity ID"
  verify unit "ambiguous mapping produces diagnostic"

}

behavior validate_rust_entity_ids "Validate Rust Entity IDs" {
  invariants [entity_mapping_precedence]
  types      [EntityMappingEntry, DiagnosticBag]

  contract """
    In strict mode, every entity ID in the mapping MUST match a declared
    entity in the spec graph. Unknown IDs MUST cause exit code 1. In
    lenient mode, unknown IDs MUST produce warnings but not fail.
  """

  verify unit "known entity ID passes in strict mode"
  verify unit "unknown entity ID fails in strict mode"
  verify unit "unknown entity ID warns in lenient mode"

}

behavior merge_workspace_reports "Merge Workspace Reports" {
  types      [SpecforgeReport, TestResultEntry]
  ports      [FileSystem]

  contract """
    The system MUST merge results from multiple test binaries in a Cargo
    workspace. Each target/specforge/*.json mapping file MUST be read.
    Duplicate entity results across binaries MUST be merged with the
    most recent result winning.
  """

  verify unit "multiple mapping files are merged"
  verify unit "duplicate entities take most recent result"

}

behavior record_test_via_drop_guard "Record Test via Drop Guard" {
  invariants [entity_mapping_precedence]
  types      [TestGuard, TestRegistry, RustFramework, RustFrameworkSupport, RustSupportLevel]

  contract """
    The #[specforge::test] proc macro MUST expand to inject a TestGuard
    that records pass/fail on Drop. The guard MUST check
    std::thread::panicking() to determine status. Results MUST be written
    to target/specforge/<binary>.json via an atexit handler. Tests using
    #[should_panic] are incompatible and MUST be documented as unsupported.
  """

  verify unit "successful test records pass via Drop"
  verify unit "panicking test records fail via Drop"
  verify unit "results written to target/specforge/ on process exit"
  verify unit "#[should_panic] incompatibility is documented"

}

behavior emit_specforge_report_from_rust "Emit Specforge Report from Rust" {
  types      [SpecforgeReport, TestResultEntry]
  ports      [FileSystem]

  contract """
    The system MUST emit a valid specforge-report.json from collected
    Rust test results. The report MUST conform to the SpecforgeReport
    schema. Entity IDs MUST be validated. The runner field MUST identify
    the Rust framework used.
  """

  verify unit "emitted report conforms to SpecforgeReport schema"
  verify unit "runner field identifies the framework"
  verify integration "full pipeline from JUnit XML to valid specforge-report.json"

}
