// @specforge/formal events — emitted by analysis passes
//
// Moved from @specforge/software events.spec per 10-expert panel.
// All IDs and type references renamed per terminology table.

use "extensions/formal/coverage-tracking"
use "extensions/formal/event-graph-linting"
use "extensions/formal/specification-layering"
use "extensions/formal/structured-conditions"
use "extensions/formal/types"

event fa_condition_check_complete "Condition Check Complete" {
  trigger   [fa_condition_check_pass]
  channel   "analysis.condition"
  payload   CoverageTrackingItem

  verify integration "event emitted after condition check pass completes"
  verify integration "payload contains violation count and coverage tracking item list"
}

event fa_layering_check_complete "Layering Check Complete" {
  trigger   [fa_layering_verify_pass]
  channel   "analysis.layering"
  payload   RefinementChain

  verify integration "event emitted after layering verify pass completes"
  verify integration "payload contains chain count and cycle violations"
  verify integration "refinement entities included in layering report"
}

event fa_event_graph_analysis_complete "Event Graph Analysis Complete" {
  trigger   [fa_event_graph_analyze_pass]
  channel   "analysis.event_graph"
  payload   EventGraphAnalysisReport
  sync      {
    barrier [fa_detect_unmitigated_cycles, fa_detect_payload_type_mismatch, fa_detect_unmatched_producers, fa_detect_unmitigated_retry_cycle, fa_detect_process_deadlock]
    timeout 30s "all event flow sub-analyses must complete within configured timeout (default: 30s)"
  }

  verify integration "event emitted after event graph analyze pass completes"
  verify integration "payload contains cycle count, retry cycle risks, and process deadlock results"
  verify deadlock_free "no circular dependency between event flow sub-analyses"
}

event fa_coverage_items_generated "Coverage Items Generated" {
  trigger   [fa_coverage_tracking_pass]
  channel   "analysis.coverage"
  payload   CoverageTrackingItem

  verify integration "event emitted after coverage tracking pass completes"
  verify integration "payload contains item breakdown by category"
  verify liveness "coverage item generation eventually completes for all entities"
}
