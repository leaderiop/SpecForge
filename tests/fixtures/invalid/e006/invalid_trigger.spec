invariant some_invariant "Some Invariant" {
  guarantee """something"""
}

event bad_trigger "Bad Trigger" {
  trigger some_invariant
  payload {
    data string
  }
}
