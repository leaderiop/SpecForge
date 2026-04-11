// specforge-test crate events
// These are in-process events, not distributed events.
// They model the lifecycle of test result collection within a single binary.

use "types"
use "behaviors"

event test_result_recorded "Test Result Recorded" {
  trigger   record_test_result_on_drop
  channel   "specforge_test.result_recorded"

  payload TestRecordEntry

  consumers [emit_binary_report]

  verify integration "recorded entry appears in binary report"
}

event binary_report_written "Binary Report Written" {
  trigger   emit_binary_report
  channel   "specforge_test.report_written"

  payload BinaryReport

  consumers [print_coverage_summary]

  verify integration "coverage summary reads the written report"
}

event graph_export_refreshed "Graph Export Refreshed" {
  trigger   invoke_specforge_export
  channel   "specforge_test.graph_refreshed"

  payload GraphExport

  consumers [generate_entity_constants, print_coverage_summary]

  verify integration "graph export is available to atexit handler"
}
