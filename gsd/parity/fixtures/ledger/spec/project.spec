project ledger "Ledger" {
  name          "Ledger"
  what          "A ledger of money on the command line: a user adds an entry in integer cents with a memo, and asks for the total"
  core_value    "A total is exact to the cent"
  context       "Built for the GSD A/B. Small enough that one person reads all of it."
  key_decisions [D-01, D-02]
  constraints   [
    { kind tech_stack what "Rust 2021, the standard library only" }
  ]
}

decision D-01 "Amounts are integer cents" {
  text      "Amounts are integer cents"
  rationale "No float rounding in a total"
  outcome   pending
  origin    project
}

decision D-02 "One entry per line, tab-separated" {
  text      "One entry per line, tab-separated"
  rationale "The store reads back with `split_once`"
  outcome   pending
  origin    project
}
