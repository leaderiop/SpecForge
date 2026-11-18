requirement LEDG-01 "Add an entry" {
  text     "A user can add an entry in integer cents with a memo"
  category "Entries"
  tier     v1
  defined  2026-11-02
}

requirement LEDG-02 "An exact total" {
  text     "The total of the entries is exact to the cent"
  category "Entries"
  tier     v1
  defined  2026-11-02
}

requirement LEDG-03 "Entries survive a restart" {
  text     "Entries survive a restart"
  category "Storage"
  tier     v1
  defined  2026-11-02
}

requirement LEDG-04 "List the entries" {
  text     "A user can list the entries in the order they were added"
  category "Listing"
  tier     v1
  defined  2026-11-02
}

requirement REPT-01 "A total per month" {
  text     "A user can see the total per month"
  category "Reports"
  tier     v2
  defined  2026-11-02
}

requirement CURR-01 "Several currencies" {
  text     "A ledger holds amounts in several currencies"
  category "Out of Scope"
  tier     out_of_scope
  reason   "A ledger is one currency"
  defined  2026-11-02
}
