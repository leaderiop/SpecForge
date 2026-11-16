// One behavior: a constraint constrains it and a failure mode affects it.

behavior write_entry "Write Entry" {
  category command
  contract "The store MUST write an entry whole."
}
