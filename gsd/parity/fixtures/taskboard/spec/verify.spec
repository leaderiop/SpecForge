uat_session 01-UAT "Phase 01 acceptance" {
  phase   v1.0/01
  source  [v1.0/01/01-01/01-01-SUMMARY, v1.0/01/01-02/01-02-SUMMARY]
  started "2026-03-03T10:30:00Z"
}

uat_test U1 "Add a task" {
  session  v1.0/01/01-UAT
  expected "`node src/cli.js add \"Write the roadmap\"` prints `added 1: Write the roadmap`"
  result   pass
}

uat_test U2 "Complete a task that does not exist" {
  session  v1.0/01/01-UAT
  expected "`node src/cli.js done 7` fails and names the id 7"
  result   issue
  reported "it prints a stack trace before the message, which is noisy"
  severity cosmetic
}

uat_test U3 "Usage text" {
  session  v1.0/01/01-UAT
  expected "`node src/cli.js` with no command prints the usage line and exits 2"
  result   pass
}

uat_gap G1 "Completing an unknown id fails with a one-line message" {
  test     v1.0/01/01-UAT/U2
  truth    "Completing an unknown id fails with a one-line message"
  reason   "User reported: it prints a stack trace before the message, which is noisy"
  severity cosmetic
}
