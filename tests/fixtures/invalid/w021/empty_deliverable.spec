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

deliverable empty_dlv "Empty Deliverable" {}
