// Milestones: one with dates, exit criteria, an owner, contributors and a dependency.

use "product/features"
use "product/modules"

milestone groundwork "Groundwork" {
  description "The store's folder layout."
  status      in_progress
}

milestone first_cut "First Cut" {
  description   "Entries are saved and listed."
  status        completed
  start_date    2026-09-01
  target_date   2026-10-15
  owner         "Alice Martin"
  contributors  ["Bob Stone"]
  features      [store_entries]
  modules       [store_module]
  depends_on    [groundwork]
  tags          ["h1"]
  exit_criteria ["An entry survives a crash", "Entries list newest first"]
}
