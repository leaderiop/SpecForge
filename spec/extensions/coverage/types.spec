// @specforge/coverage extension types
// Coverage types are entity-agnostic — they work with any testable entity
// kind declared by extensions via the KindRegistry testable flag.

type CoverageReport {
  entities   EntityCoverageResult[]
  summary    CoverageSummary
}

type EntityCoverageResult {
  entity_id   string   @readonly
  entity_kind string   @readonly
  status      TestStatus
  duration_ms number   @optional
  test_file   string   @optional
  test_name   string   @optional
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
}

type SpecforgeReport {
  specforge    string    @readonly
  runner       string
  timestamp    string
  results      TestResultEntry[]
}

type TestResultEntry {
  entity_id    string    @readonly
  test_file    string
  tests        TestResult[]
}

type TestResult {
  name         string
  status       TestResultStatus
  duration_ms  number    @optional
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
}

type CoverageConfig {
  threshold      number   @optional
  fail_on_unknown_ids boolean @optional
  test_dirs      string[] @optional
}
