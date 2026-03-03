define risk_register {
  severity integer
  description string
}

risk_register bad_risk "Bad Risk" {
  severity "not_a_number"
  description "This should fail type check"
}
