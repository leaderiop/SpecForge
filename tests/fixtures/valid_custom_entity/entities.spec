define risk_register {
  severity integer
  likelihood integer
  description string
}

risk_register auth_risk "Auth Risk" {
  severity 8
  likelihood 3
  description "Authentication bypass"
}

risk_register data_risk "Data Risk" {
  severity 5
  likelihood 2
  description "Data corruption"
}
