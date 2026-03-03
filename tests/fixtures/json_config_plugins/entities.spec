invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated"""
  risk low
}

behavior validate_input "Validate Input" {
  invariants [data_integrity]
  contract """the system MUST validate all input fields"""
  verify unit "check validation logic"
}

feature input_validation "Input Validation" {
  behaviors [validate_input]
  problem """invalid data can corrupt the system"""
  solution """validate all input before processing"""
  status active
}

decision use_postgresql "Use PostgreSQL" {
  status accepted
  context """need a reliable RDBMS"""
  rationale """proven track record, strong community"""
  adrs [use_postgresql]
}
