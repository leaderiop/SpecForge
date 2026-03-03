spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}

invariant high_risk_safety "High Risk" {
  guarantee """data MUST be safe"""
  risk high
}
