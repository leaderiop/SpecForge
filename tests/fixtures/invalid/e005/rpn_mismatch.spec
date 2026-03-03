spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}

invariant data_safety "Data Safety" {
  guarantee """data must be safe"""
  risk high
}

failure_mode data_loss "Data Loss" {
  severity 5
  occurrence 3
  detection 2
  rpn 99
  mitigates [data_safety]
}
