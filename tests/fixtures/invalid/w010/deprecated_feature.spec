spec "test" {
  version "1.0"
  plugins ["@specforge/product"]
}

feature old_feature "Old Feature" {
  problem """no longer needed"""
  solution """deprecated"""
  status deprecated
  behaviors [legacy_behavior]
}

behavior legacy_behavior "Some Behavior" {
  contract """must work"""
  verify unit "test it"
}
