uat_session 01-UAT "Phase 01 acceptance" {
  phase   v1.0/01
  source  [v1.0/01/01-01/01-01-SUMMARY]
  started "2026-10-29T10:00:00Z"
}

uat_test U1 "Add a task" {
  session   v1.0/01/01-UAT
  expected  "A task added from the form is listed"
  criterion 1
}

uat_test U2 "Complete a task" {
  session   v1.0/01/01-UAT
  expected  "A completed task is struck through"
  criterion 2
}

uat_gap G1 "Completed tasks are not struck through" {
  test       v1.0/01/01-UAT/U2
  truth      "A completed task is struck through"
  reason     "The list ignores the completed flag"
  severity   major
  root_cause "complete() is never read by the renderer"
}
