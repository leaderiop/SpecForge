verification 01-VERIFICATION "Phase 01 verification" {
  phase v1.0/01
}

verify_gap VG1 "Tasks cannot be listed" {
  verification v1.0/01/01-VERIFICATION
  truth        v1.0/01/01-01/TR1
  reason       "list() returns nothing after add()"
  missing      ["list() ordering"]
}

override OV1 "Accept the unverified listing" {
  verification v1.0/01/01-VERIFICATION
  truth        v1.0/01/01-01/TR1
  reason       "Checked by hand against the running app"
  by           "ada"
  at           2026-10-29
}

human_item HI1 "The listing reads well" {
  verification v1.0/01/01-VERIFICATION
  test         "Open the task list"
  expected     "Tasks appear newest first"
  why_human    "Readability is a judgement"
}
