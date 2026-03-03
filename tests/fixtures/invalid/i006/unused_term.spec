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

glossary {
  term "obscure_concept" {
    definition """A concept that nobody references anywhere"""
  }
}

behavior create_user "Create User" {
  contract """the system MUST create a user"""
  verify unit "check user creation"
}
