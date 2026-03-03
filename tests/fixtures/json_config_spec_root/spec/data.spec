invariant data_integrity "Data Integrity" {
  guarantee """all data MUST be validated before persistence"""
  risk high
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
