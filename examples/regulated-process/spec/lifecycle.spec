// The batch lifecycle as a state machine (@specforge/formal).

process batch_lifecycle "Batch lifecycle" {
  description   "The states a batch moves through from manufacture to market or destruction."
  alphabet      ["batch_record_reviewed", "batch_certified", "batch_rejected", "batch_released"]
  states        ["quarantine", "reviewed", "certified", "released", "rejected"]
  initial_state "quarantine"
  composition   "parallel"
  sub_processes [deviation_handling]
}

process deviation_handling "Deviation handling" {
  description   "Each deviation on a batch runs alongside the batch lifecycle until it is closed."
  alphabet      ["deviation_closed"]
  states        ["open", "closed"]
  initial_state "open"
}

protocol qa_qp_handover "QA to QP handover" {
  description   "QA hands a reviewed batch to the QP, who answers with a certification or a rejection."
  alphabet      ["batch_record_reviewed", "batch_certified", "batch_rejected"]
  states        ["waiting_for_qa", "with_qp", "decided"]
  initial_state "waiting_for_qa"
  transitions   [
    "waiting_for_qa -> batch_record_reviewed -> with_qp",
    "with_qp -> batch_certified -> decided",
    "with_qp -> batch_rejected -> decided",
  ]
}
