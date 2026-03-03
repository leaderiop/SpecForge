behavior validate_input "Validate Input" {
  contract """input MUST be validated before processing"""
  verify unit "checks invalid input"
  tests ["nonexistent_test.rs"]
}
