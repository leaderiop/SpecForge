behavior step_one "Step One" {
  contract """The system MUST perform step one."""
  verify unit "runs step one"
}

behavior step_two "Step Two" {
  contract """The system MUST perform step two."""
  verify unit "runs step two"
}

behavior step_three "Step Three" {
  contract """The system MUST perform step three."""
  verify unit "runs step three"
}

invariant step_safety "Step Safety" {
  guarantee """Steps MUST execute in order."""
  enforced_by [step_one]
}

invariant step_idempotent "Step Idempotent" {
  guarantee """Steps MUST be idempotent."""
  enforced_by [step_two]
}
