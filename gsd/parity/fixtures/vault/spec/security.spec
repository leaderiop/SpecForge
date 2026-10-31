security_audit 01-SECURITY "Phase 1 security" {
  phase v1.0/01
}

accepted_risk AR1 "Names are not secret" {
  audit       v1.0/01/01-SECURITY
  threat      v1.0/01/01-02/T-01-03
  rationale   "the names are not secret; the values are never printed"
  accepted_by "maintainer"
  at          2026-10-30
}

audit_run AU1 "Audit of 2026-10-29" {
  audit  v1.0/01/01-SECURITY
  at     "2026-10-29T10:00:00Z"
  total  3
  closed 1
  open   2
  run_by "gsd-security-auditor"
}
