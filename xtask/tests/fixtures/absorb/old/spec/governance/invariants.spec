// Invariants without obligations: one bare, one with a description and refs.

invariant writes_are_durable "Writes Are Durable" {
  guarantee "An acknowledged write survives a crash."
  risk      high
}

invariant entries_are_whole "Entries Are Whole" {
  guarantee   "An entry is read back whole or not at all."
  risk        low
  description "No reader ever sees half an entry."
  refs        ["Storage design note"]
}
