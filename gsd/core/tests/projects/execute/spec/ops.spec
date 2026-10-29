pause PS1 "Paused before the storage decision" {
  phase   v1.0/01
  plan    v1.0/01/01-02
  reason  "How completion is stored is still open"
  handoff v1.0/01/HO1
}

handoff HO1 "Completion storage" {
  phase          v1.0/01
  task           v1.0/01/01-02/T1
  state          in_progress
  current_state  "complete() is written, its storage is not"
  completed_work [
    { task v1.0/01/01-01/T1 state done }
  ]
  remaining_work ["Pick how completion is stored"]
  decisions_made [v1.0/01/D-01]
  blockers       [BL1]
  next_action    "Answer checkpoint C1 of plan 01-02"
}

blocker BL1 "Completion storage is undecided" {
  text  "How completion is stored is undecided"
  phase v1.0/01
  plan  v1.0/01/01-02
  resolved {
    at  "2026-10-29T10:00:00Z"
    how "Decided: a timestamp"
  }
}
