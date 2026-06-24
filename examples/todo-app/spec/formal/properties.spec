// Formal methods: a single temporal property (a small taste).

property no_lost_completion "Completion is never lost" {
  property_type safety
  expression """
    Once a task reaches status `done`, it never silently reverts to a
    non-done status without an explicit reopen command.
  """
  scope "task lifecycle"
}
