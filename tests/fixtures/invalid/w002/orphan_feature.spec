spec "test" {
  version "1.0"
  plugins ["@specforge/product"]
}

feature lonely_feature "Lonely Feature" {
  problem """this feature is not referenced by any capability"""
  solution """nothing"""
  status active
  behaviors [handle_request]
}

behavior handle_request "Some Behavior" {
  contract """must work"""
  verify unit "test it"
}
