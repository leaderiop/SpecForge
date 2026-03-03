// Test coverage types

type CoverageReport {
  behaviors  BehaviorResult[]
  invariants InvariantResult[]
  summary    CoverageSummary
}

type BehaviorResult {
  behaviorId  string   @readonly
  status      TestStatus
  duration    number   @optional
  testFile    string   @optional
  testName    string   @optional
}

type TestStatus = pass | fail | skip | missing

type InvariantResult {
  invariantId string  @readonly
  hasViolationTest boolean
  testFile    string  @optional
}

type CoverageSummary {
  totalTestable      integer
  declaredCount      integer
  linkedCount        integer
  executedCount      integer
  passingCount       integer
  declaredPercent    number
  linkedPercent      number
  executedPercent    number
  passingPercent     number
}

type SpecforgeReport {
  specforge    string    @readonly
  runner       string
  timestamp    timestamp
  results      TestResultEntry[]
}

type TestResultEntry {
  entityId     string    @readonly
  file         string
  tests        TestResult[]
}

type TestResult {
  name         string
  status       TestResultStatus
  durationMs   number    @optional
}

type TestResultStatus = pass | fail

type CoverageLevel = declared | linked | executed | passing
