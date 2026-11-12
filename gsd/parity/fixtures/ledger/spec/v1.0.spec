milestone v1.0 "Ledger 1.0" {
  name "Ledger 1.0"
}

phase 01 "Core" {
  milestone        v1.0
  name             "Core"
  slug             "core"
  goal             "Entries can be added and totalled"
  requirements     [LEDG-01, LEDG-02]
  success_criteria [
    "A user can add an entry in integer cents with a memo",
    "The total of the entries is exact to the cent",
  ]
}

phase 02 "Persistence" {
  milestone        v1.0
  name             "Persistence"
  slug             "persistence"
  goal             "Entries are saved to disk and read back"
  depends_on       [01]
  requirements     [LEDG-03]
  success_criteria [
    "An added entry is written to `ledger.tsv`",
    "A restarted ledger shows the entries it had",
  ]
}

phase 03 "Listing" {
  milestone        v1.0
  name             "Listing"
  slug             "listing"
  goal             "Entries are listed in the order they were added"
  depends_on       [02]
  requirements     [LEDG-04]
  success_criteria [
    "`ledger list` shows every entry, oldest first",
    "An empty ledger lists nothing and says so",
  ]
}

verification 01-VERIFICATION "Phase 01 verification" {
  phase v1.0/01
}
