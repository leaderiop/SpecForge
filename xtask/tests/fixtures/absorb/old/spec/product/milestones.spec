// Milestones: one in no release; one with dates, exit criteria, an owner, contributors and a dependency; one
// depending on it in its release, and one depending on it from the next release.

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

milestone second_cut "Second Cut" {
  description   "Entries are listed."
  status        planned
  features      [list_entries]
  depends_on    [first_cut]
  exit_criteria ["Entries list in one call", "An empty store lists nothing"]
}

milestone third_cut "Phase 3: Third Cut" {
  description   "Entries are shared."
  status        planned
  depends_on    [first_cut]
  exit_criteria ["A shared entry is read by another", "A private entry is not"]
}
