// @specforge/coverage extension events

use "extensions/coverage/types"
event test_report_consumed "Test Report Consumed" {
  channel   "coverage.report_consumed"

  payload TestReportConsumedPayload


  verify integration "emits test_report_consumed with correct four-level coverage counts"
  verify integration "consumers compute four-level coverage and render traceability matrix"

}
