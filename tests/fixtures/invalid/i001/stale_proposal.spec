spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}

decision pending_decision "Pending Decision" {
  status proposed
  date 2026-01-01
  context """still thinking"""
  decision """not decided yet"""
}
