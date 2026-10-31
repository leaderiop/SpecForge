trust_boundary TB1 "Browser to API" {
  plan          v1.0/01/01-01
  boundary      "Browser to API"
  description   "Untrusted task titles reach the store"
  data_crossing "task titles (user input)"
}

threat T-01-01 "Injected title" {
  plan        v1.0/01/01-01
  category    tampering
  component   "src/tasks.js"
  severity    high
  disposition mitigate
  mitigation  "Titles are stored as data and never evaluated"
  files       ["src/tasks.js"]
}

threat T-01-SC "Supply chain" {
  plan        v1.0/01/01-01
  category    tampering
  component   "dependencies"
  severity    low
  disposition accept
  mitigation  "No dependencies"
}

security_audit 01-SECURITY "Phase 01 security" {
  phase v1.0/01
}

audit_run AU1 "First run" {
  audit  v1.0/01/01-SECURITY
  at     "2026-10-30T09:00:00Z"
  total  2
  closed 1
  open   1
  run_by "gsd-security-auditor"
}

accepted_risk AR1 "No dependencies to pin" {
  audit       v1.0/01/01-SECURITY
  threat      v1.0/01/01-01/T-01-SC
  rationale   "Nothing to attack"
  accepted_by "ada"
  at          2026-10-30
}
