// @specforge/coverage extension events

use extensions/coverage/types
use extensions/coverage/behaviors

event test_report_consumed "Test Report Consumed" {
  trigger   consume_specforge_report
  channel   "coverage.report_consumed"

  payload TestReportConsumedPayload

  consumers [compute_four_level_coverage, render_test_traceability_matrix]

  verify integration "emits test_report_consumed with correct four-level coverage counts"
  verify integration "consumers compute four-level coverage and render traceability matrix"

}
