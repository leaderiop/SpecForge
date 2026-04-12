// @specforge/software events — emitted by analysis passes

use "extensions/software/formal-contracts"
use "extensions/software/types"
event se_contract_check_complete "Contract Check Complete" {
  channel "analysis.contract"
  payload ProofObligation

  verify integration "event emitted after contract check pass completes"
  verify integration "payload contains violation count and proof obligation list"

}

event se_refinement_check_complete "Refinement Check Complete" {
  channel "analysis.refinement"
  payload RefinementChain

  verify integration "event emitted after refinement verify pass completes"
  verify integration "payload contains chain count and cycle violations"

}

event se_concurrency_analysis_complete "Concurrency Analysis Complete" {
  channel "analysis.concurrency"
  payload ConcurrencyAnalysisReport

  sync {
    barrier [se_detect_event_deadlocks, se_detect_channel_type_mismatch, se_detect_unmatched_producers, se_detect_livelock_risk]
    timeout 30s "all concurrency sub-analyses must complete within 30 seconds"
  }

  verify integration "event emitted after process analyze pass completes"
  verify integration "payload contains deadlock count and livelock risks"
  verify deadlock_free "no circular dependency between concurrency sub-analyses"

}

event se_proof_obligations_generated "Proof Obligations Generated" {
  channel "analysis.proof"
  payload ProofObligation

  verify integration "event emitted after proof obligation pass completes"
  verify integration "payload contains obligation breakdown by category"
  verify liveness "proof obligation generation eventually completes for all entities"

}
