spec "test" {
  version "1.0"
  plugins ["@specforge/product"]

  persona developer "Developer" {
    description "Writes code"
  }

  surface cli "CLI" {
    type terminal
  }
}

capability orphan_capability "Orphan Capability" {
  persona developer
  surface cli
  features [basic_feature]
}

feature basic_feature "Some Feature" {
  problem """something"""
  solution """something"""
  status active
  behaviors [basic_behavior]
}

behavior basic_behavior "Some Behavior" {
  contract """must work"""
  verify unit "test it"
}
