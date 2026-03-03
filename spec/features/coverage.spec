// Test coverage feature

use behaviors/coverage

feature test_coverage_reporting "Test Coverage Reporting" {
  behaviors [merge_coverage_reports, compute_coverage_summary, gate_on_coverage_threshold, validate_test_ids_against_spec]

  problem """
    Teams need to know which behaviors have tests, which are missing
    coverage, and whether the project meets its coverage threshold.
    This data must come from actual test runs, not just spec declarations.
  """

  solution """
    specforge coverage merges specforge-report.json files from test runners,
    computes coverage statistics, gates on configurable thresholds, and
    validates that test IDs reference real behaviors. Framework-native
    plugins (vitest, pytest, go) produce the reports.
  """
}

feature test_traceability "Test Traceability" {
  behaviors [consume_specforge_report, compute_three_layer_coverage, render_test_traceability_matrix, validate_tests_field_references]

  problem """
    Teams need end-to-end proof that spec entities are implemented
    and tested. Knowing that a behavior has a verify statement is not
    enough — there must be a linked test file and actual execution
    results to close the traceability loop.
  """

  solution """
    Three-layer traceability model: intent (verify/scenario declarations),
    linkage (tests field pointing to real files), and proof
    (specforge-report.json with pass/fail results). specforge trace
    --test-results renders the full matrix.
  """
}
