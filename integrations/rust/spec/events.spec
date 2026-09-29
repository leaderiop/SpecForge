// specforge-test crate events
// These are in-process events, not distributed events.
// They model the lifecycle of test result collection within a single binary.

use "types"

event test_result_recorded "Test Result Recorded" {
  channel   "specforge_test.result_recorded"

  payload TestRecordEntry


  verify integration "recorded entry appears in binary report"
}

event binary_report_written "Binary Report Written" {
  channel   "specforge_test.report_written"

  payload BinaryReport


  verify integration "coverage summary reads the written report"
}

event graph_export_refreshed "Graph Export Refreshed" {
  channel   "specforge_test.graph_refreshed"

  payload GraphExport


  verify integration "graph export is available to atexit handler"
}
