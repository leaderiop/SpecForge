validation 01-VALIDATION "Phase 01 validation" {
  phase         v1.0/01
  framework     "node:test"
  quick_cmd     "node --test"
  full_cmd      "node --test test/"
  max_latency_s 30
  approval {
    by "ada"
    at "2026-10-29T11:00:00Z"
  }
}

validation_row VR1 "The store is tested" {
  validation  v1.0/01/01-VALIDATION
  task        v1.0/01/01-01/T2
  requirement TASK-01
  test_type   unit
  command     "node --test test/tasks.test.js"
}

wave0_item W0-1 "The store test stub" {
  validation v1.0/01/01-VALIDATION
  path       "test/tasks.test.js"
  for        TASK-01
  kind       test_stub
}

manual_check MC1 "The list reads well" {
  validation   v1.0/01/01-VALIDATION
  behavior     "The task list is readable"
  requirement  TASK-01
  why_manual   "Readability is a judgement"
  instructions "Open the list and read it"
}
