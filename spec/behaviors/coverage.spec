// Test coverage behaviors — reporting and gating

use invariants/core
use types/coverage
use types/diagnostics
use ports/outbound
use governance/decisions

behavior merge_coverage_reports "Merge Coverage Reports" {
  types      [CoverageReport, BehaviorResult]
  ports      [TestReporter, FileSystem]

  contract """
    When specforge coverage is invoked, the system MUST locate and merge
    all specforge-report.json files from configured test directories.
    Each behavior MUST appear at most once in the merged report. If a
    behavior appears in multiple reports, the most recent result MUST win.
  """

  verify unit        "single report is loaded correctly"
  verify unit        "multiple reports are merged"
  verify unit        "duplicate behavior takes most recent result"
  verify integration "reports from configured test_dirs are discovered"
}

behavior compute_coverage_summary "Compute Coverage Summary" {
  types      [CoverageReport, CoverageSummary, InvariantResult, TestResult, TestResultStatus, TestStatus]
  adrs       [snapshot_testing_with_insta]

  contract """
    The system MUST compute coverage statistics from merged reports:
    total behaviors (from .spec files), covered behaviors (with test
    results), coverage percentage, and per-status counts (pass, fail,
    skip, missing).
  """

  verify unit "coverage percentage is correct"
  verify unit "missing behaviors are counted"
  verify unit "all status categories are tallied"
}

behavior gate_on_coverage_threshold "Gate on Coverage Threshold" {
  types      [CoverageSummary, CoverageConfig]

  contract """
    When specforge coverage --min=N is invoked, the system MUST compare
    the computed coverage percentage against N. If coverage is below the
    threshold, the command MUST exit with code 1. The threshold MAY also
    be set via the coverage.threshold field in the spec root.
  """

  verify unit "coverage above threshold exits 0"
  verify unit "coverage below threshold exits 1"
  verify unit "threshold from spec root is used as default"
}

behavior validate_test_ids_against_spec "Validate Test IDs Against Spec" {
  types      [CoverageReport, DiagnosticBag]

  contract """
    When fail_on_unknown_ids is enabled, the system MUST validate that
    every behavior ID in specforge-report.json files matches a declared
    behavior in the .spec files. Unknown IDs MUST cause the test suite
    to fail.
  """

  verify unit "known behavior ID passes"
  verify unit "unknown behavior ID fails when gate is enabled"
  verify unit "unknown IDs are ignored when gate is disabled"
}

behavior consume_specforge_report "Consume Specforge Report" {
  types      [SpecforgeReport, TestResultEntry]
  ports      [TestReporter, FileSystem]

  contract """
    The system MUST parse specforge-report.json files and validate them
    against the SpecforgeReport schema. Each entity ID in the report
    MUST be matched against the spec graph. Unmatched IDs MUST be
    collected for diagnostic reporting.
  """

  verify unit "valid report is parsed successfully"
  verify unit "report with unknown entity IDs collects unmatched entries"
  verify unit "malformed report produces parse error"
  verify integration "report files are discovered from configured directories"
}

behavior compute_three_layer_coverage "Compute Three-Layer Coverage" {
  types      [CoverageSummary, CoverageLevel]

  contract """
    The system MUST compute four coverage levels from the spec graph
    and report data: declared (has verify/scenario), linked (has tests
    field), executed (appears in report), and passing (all tests pass).
    Each level MUST be expressed as both a count and a percentage of
    total testable entities.
  """

  verify unit "declared count matches entities with verify or scenario"
  verify unit "linked count matches entities with tests field"
  verify unit "executed count matches entities in report"
  verify unit "passing count matches entities with all tests passing"
}

behavior render_test_traceability_matrix "Render Test Traceability Matrix" {
  types      [CoverageSummary, TestResultEntry]

  contract """
    The system MUST render the specforge trace --test-results output
    as a tabular display showing each testable entity with its intent
    declarations (verify/scenario count), linked test file, and test
    execution status from the report.
  """

  verify unit "matrix includes all testable entities"
  verify unit "matrix shows correct status for each coverage level"
  verify integration "full trace output matches expected tabular format"
}
