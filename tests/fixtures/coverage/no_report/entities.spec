behavior declared_only "Declared Only" {
  contract """The system MUST do something."""
  verify unit "does something"
}

behavior linked_too "Linked Too" {
  contract """The system MUST also do this."""
  verify unit "does this too"
  tests "tests/linked.test.ts"
}

invariant some_rule "Some Rule" {
  guarantee """This rule MUST hold."""
  enforced_by [declared_only]
}
