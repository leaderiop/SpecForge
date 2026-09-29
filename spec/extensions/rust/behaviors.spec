// @specforge/rust extension behaviors — linking Rust tests to spec entities
//
// Running the tests and reading their reports belongs to
// @specforge/cargo-test (spec/extensions/cargo-test); these behaviors
// cover how a Rust test names the entity it proves.

use "invariants/core"
use "extensions/rust/invariants"
use "types/core"
use "extensions/rust/types"
use "extensions/rust/decisions"
behavior resolve_entity_mapping "Resolve Entity Mapping" {
  types      [EntityMappingEntry, MappingResolutionLevel]
  category   query
  invariants [entity_mapping_precedence]

  contract """
    The system MUST resolve test-to-entity mappings using two-level
    precedence: (1) the #[specforge_test] proc macro attribute
    (explicit), (2) module name / double-underscore convention
    (implicit). The explicit level MUST override the implicit one.
    Ambiguous mappings MUST be reported. Spec files carry no test paths:
    the `tests` field is retired (ADR 0002).
  """

  verify unit "proc macro attribute overrides naming convention"
  verify unit "double-underscore convention extracts entity ID"
  verify unit "ambiguous mapping produces diagnostic"
}

behavior record_test_via_drop_guard "Record Test via Drop Guard" {
  invariants [entity_mapping_precedence]
  category   command
  types      [TestGuard, TestRegistry, RustFramework, RustFrameworkSupport, RustSupportLevel]

  contract """
    The #[specforge_test] proc macro MUST register the annotated function
    as a test (no separate #[test]; it defers to a runner attribute such
    as #[tokio::test] below it) and inject a TestGuard that records
    pass/fail on Drop. The guard MUST check std::thread::panicking() to
    determine status, inverted under #[should_panic]; an #[ignore]d test
    is recorded as skipped. Results MUST be written via an atexit handler
    to $SPECFORGE_REPORT when `specforge collect` sets it, else to
    target/specforge/, one report per test target named
    <package>--<target>.json so a rebuild replaces its predecessor.
  """

  verify unit "successful test records pass via Drop"
  verify unit "panicking test records fail via Drop"
  verify unit "results written to target/specforge/ on process exit"
  verify unit "the attribute registers the test without #[test]"
}
