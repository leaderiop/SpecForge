plan 261105-tcl "Show totals in a fixed-width column" {
  type             execute
  slug             "totals-column"
  files_modified   ["src/lib.rs", "src/main.rs", "tests/totals.rs"]
  objective        "Show totals in a fixed-width column."
  purpose          "totals line up when a user prints several stores one under another."
  output           "`column(cents)` in `src/lib.rs`, used by `ledger total`."
  success_criteria ["A total prints right-aligned in a column of fourteen"]
}

task T1 "The total's column" {
  plan        261105-tcl
  type        auto
  files       ["src/lib.rs", "src/main.rs", "tests/totals.rs"]
  items       ["rust:ledger::column"]
  action      "Add `column(cents)`: euros and cents, right-aligned in 14 characters; `ledger total` prints through it. Fold the total in an `i32` so the column's width is the value's width."
  verify_text "cargo test --test totals a_total_prints_in_a_column_of_fourteen"
  done        "A total prints in a column of fourteen"
}

truth TR1 "A total prints right-aligned in a column of fourteen" {
  plan  261105-tcl
  text  "A total prints right-aligned in a column of fourteen"
  proof ["rust:ledger::totals#a_total_prints_in_a_column_of_fourteen"]
}

artifact AR1 "column" {
  plan     261105-tcl
  path     "src/lib.rs"
  provides "column"
  contains "pub fn column"
}

summary 261105-tcl-SUMMARY "Show totals in a fixed-width column: summary" {
  plan                   261105-tcl
  status                 complete
  one_liner              "`ledger total` prints its total right-aligned in a column of fourteen."
  subsystem              "entries"
  requirements_completed []
}
