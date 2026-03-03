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

capability empty_capability "Empty Capability" {
  persona developer
  surface cli
}
