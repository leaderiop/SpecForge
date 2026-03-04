spec "collect-test" {
  version "1.0"
  plugins []
}

behavior validate_input "Validate Input" {
  contract """the system MUST validate all input fields"""
  verify unit "rejects empty email"
  verify unit "accepts valid email"
}

behavior data_integrity "Data Integrity" {
  contract """data MUST remain consistent"""
  verify property "holds under mutation"
}
