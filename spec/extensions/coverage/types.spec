// @specforge/coverage extension types
// Coverage types are entity-agnostic — they work with any testable entity
// kind declared by extensions via the KindRegistry testable flag.

type CoverageReport {
  entities   EntityCoverageResult[]
  summary    CoverageSummary
  verify unit "CoverageReport schema is valid"
}

type EntityCoverageResult {
  entity_id   string   @readonly
  entity_kind string   @readonly
  status      TestStatus
  duration_ms number   @optional
  test_file   string   @optional
  test_name   string   @optional
  verify unit "EntityCoverageResult schema is valid"
}

type TestStatus = pass | fail | skip | missing

type CoverageSummary {
  total_testable     integer
  declared_count     integer
  linked_count       integer
  executed_count     integer
  passing_count      integer
  declared_percent   number
  linked_percent     number
  executed_percent   number
  passing_percent    number
  verify unit "CoverageSummary schema is valid"
}

type SpecforgeReport {
  specforge    string    @readonly
  runner       string
  timestamp    string
  results      TestResultEntry[]
  verify unit "SpecforgeReport schema is valid"
}

type TestResultEntry {
  entity_id    string    @readonly
  test_file    string
  tests        TestResult[]
  verify unit "TestResultEntry schema is valid"
}

type TestResult {
  name         string
  status       TestResultStatus
  duration_ms  number    @optional
  verify unit "TestResult schema is valid"
}

// TestResultStatus covers individual test outcomes from test runners.
// skip is included because test runners report skipped/ignored tests
// (e.g., #[ignore] in Rust, @skip in Jest). missing is NOT included
// because it is a coverage-level state (entity has no test results),
// not something a test runner reports.
type TestResultStatus = pass | fail | skip

type CoverageLevel = declared | linked | executed | passing

type TestReportConsumedPayload {
  entityCount    integer
  matchedCount   integer
  unmatchedCount integer
  declaredCount  integer
  linkedCount    integer
  executedCount  integer
  passingCount   integer
  timestamp      timestamp
  verify unit "TestReportConsumedPayload schema is valid"
}

// CoverageConfig is the canonical configuration type for the coverage
// extension. Fields merged from the former core CoverageConfig:
//   - reports: output report format names (e.g., "summary", "json", "lcov")
//   - require_violation_tests: when true, invariant-violating entities must have tests
type CoverageConfig {
  threshold                number    @optional
  reports                  string[]  @optional
  require_violation_tests  boolean   @optional
  fail_on_unknown_ids      boolean   @optional
  test_dirs                string[]  @optional
  verify unit "CoverageConfig schema is valid"
}
