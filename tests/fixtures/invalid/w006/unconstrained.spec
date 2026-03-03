spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}

behavior unconstrained_action "No Constraints" {
  contract """this behavior has no constraints"""
  verify unit "test something"
}

feature some_feature "Feature" {
  behaviors [unconstrained_action]
  problem """need it"""
  solution """build it"""
  status active
}
